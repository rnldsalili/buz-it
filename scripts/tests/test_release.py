import importlib.util
import pathlib
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location('release', pathlib.Path(__file__).parents[1] / 'release.py')
release = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(release)

class VersionTests(unittest.TestCase):
    def test_bumps(self):
        for kind, expected in [('patch','0.1.1'),('minor','0.2.0'),('major','1.0.0')]:
            self.assertEqual(release.bump('0.1.0', kind), expected)

    def test_invalid_versions(self):
        for version in ['01.1.0','v1.0.0','1.0','1.0.0-beta','-1.0.0']:
            with self.assertRaises(ValueError): release.bump(version, 'patch')
        with self.assertRaises(ValueError): release.bump('1.0.0', 'oops')

    def fixture(self, root):
        (root/'src-tauri').mkdir()
        (root/'crates/core').mkdir(parents=True)
        (root/'Cargo.toml').write_text('[workspace]\nmembers = ["crates/core", "src-tauri"]\n[workspace.package]\nversion = "0.1.0"\n')
        (root/'src-tauri/tauri.conf.json').write_text('{"version":"0.1.0","productName":"Buz It"}\n')
        for directory, name in [('src-tauri','buz-it'),('crates/core','buz-it-core')]:
            (root/directory/'Cargo.toml').write_text(f'[package]\nname = "{name}"\nversion.workspace = true\n')
        (root/'Cargo.lock').write_text('version = 4\n\n[[package]]\nname = "buz-it"\nversion = "0.1.0"\n\n[[package]]\nname = "buz-it-core"\nversion = "0.1.0"\n\n[[package]]\nname = "other"\nversion = "0.1.0"\nsource = "registry+example"\nchecksum = "unchanged"\n')

    def test_sync_preserves_dependencies(self):
        with tempfile.TemporaryDirectory() as directory:
            root=pathlib.Path(directory); self.fixture(root)
            before=(root/'Cargo.lock').read_text().split('name = "other"')[1]
            release.set_version(root, '0.2.0')
            self.assertEqual(release.read_version(root), '0.2.0')
            self.assertEqual((root/'Cargo.lock').read_text().split('name = "other"')[1], before)
            release.check_versions(root, '0.2.0')

    def test_mismatch_does_not_partially_write(self):
        with tempfile.TemporaryDirectory() as directory:
            root=pathlib.Path(directory); self.fixture(root)
            (root/'src-tauri/tauri.conf.json').write_text('{"version":"0.9.0"}')
            before=(root/'Cargo.toml').read_bytes()
            with self.assertRaises(ValueError): release.set_version(root,'0.1.1')
            self.assertEqual((root/'Cargo.toml').read_bytes(),before)


class GitReleaseTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = pathlib.Path(self.temp.name)
        self.root = self.base/'repo'; self.root.mkdir()
        self.remote = self.base/'remote.git'
        self.candidate = self.base/'candidate'
        VersionTests().fixture(self.root)
        release.command('git','init','-b','main',cwd=self.root)
        release.command('git','config','user.name','Test',cwd=self.root)
        release.command('git','config','user.email','test@example.com',cwd=self.root)
        release.command('git','add','.',cwd=self.root)
        release.command('git','commit','-m','initial',cwd=self.root)
        release.command('git','init','--bare',str(self.remote))
        release.command('git','remote','add','origin',str(self.remote),cwd=self.root)
        release.command('git','push','origin','main',cwd=self.root)
        self.initial = release.remote_ref(self.root,'refs/heads/main')

    def prepare(self):
        release.prepare(self.root,self.candidate,'patch')
        return release.load_candidate(self.candidate)

    def test_candidate_is_unpublished_and_reused_exactly(self):
        data=self.prepare()
        self.assertEqual(release.remote_ref(self.root,'refs/heads/main'),self.initial)
        self.assertIsNone(release.remote_ref(self.root,'refs/tags/v0.1.1'))
        release.command('git','checkout','--detach',self.initial,cwd=self.root)
        with self.assertRaises(ValueError): release.prepare(self.root,self.candidate,'major')
        release.prepare(self.root,self.candidate,'patch')
        self.assertEqual(release.load_candidate(self.candidate),data)
        clone=self.base/'build'
        release.command('git','clone','--branch','main',str(self.remote),str(clone))
        release.restore(clone,self.candidate)
        self.assertEqual(release.command('git','rev-parse','HEAD',cwd=clone),data['commit'])
        release.check_versions(clone,'0.1.1')

    def test_atomic_push_and_retry(self):
        data=self.prepare()
        release.push_candidate(self.root,data)
        for ref in ['refs/heads/main','refs/tags/v0.1.1']:
            self.assertEqual(release.remote_ref(self.root,ref),data['commit'])
        release.push_candidate(self.root,data)

    def test_main_advance_prevents_publication(self):
        data=self.prepare()
        release.command('git','checkout','--detach',self.initial,cwd=self.root)
        (self.root/'new.txt').write_text('new work')
        release.command('git','add','new.txt',cwd=self.root)
        release.command('git','commit','-m','main advanced',cwd=self.root)
        release.command('git','push','origin','HEAD:main',cwd=self.root)
        with self.assertRaisesRegex(ValueError,'main advanced'):
            release.push_candidate(self.root,data)
        self.assertIsNone(release.remote_ref(self.root,'refs/tags/v0.1.1'))

    def test_tag_conflict_is_not_overwritten(self):
        data=self.prepare()
        release.command('git','tag','v0.1.1',self.initial,cwd=self.root)
        release.command('git','push','origin','refs/tags/v0.1.1',cwd=self.root)
        with self.assertRaisesRegex(ValueError,'different commit'):
            release.push_candidate(self.root,data)
        self.assertEqual(release.remote_ref(self.root,'refs/tags/v0.1.1'),self.initial)

    def test_retry_after_push_failure(self):
        data=self.prepare()
        hook=self.remote/'hooks/pre-receive'
        if __import__('os').name == 'nt': self.skipTest('Unix Git hook fixture')
        hook.write_text('#!/bin/sh\nexit 1\n'); hook.chmod(0o755)
        with self.assertRaises(__import__('subprocess').CalledProcessError):
            release.push_candidate(self.root,data)
        self.assertEqual(release.remote_ref(self.root,'refs/heads/main'),self.initial)
        hook.unlink()
        release.push_candidate(self.root,data)
        self.assertEqual(release.remote_ref(self.root,'refs/tags/v0.1.1'),data['commit'])

    def test_untrusted_candidate_commit_is_rejected(self):
        self.prepare()
        with self.assertRaisesRegex(ValueError,'trusted preparation commit'):
            release.restore(self.root,self.candidate,expected_commit=self.initial)

    def test_reused_candidate_rejects_extra_config_changes(self):
        import hashlib, json
        data=self.prepare()
        config=self.root/'src-tauri/tauri.conf.json'
        content=json.loads(config.read_text()); content['productName']='Unexpected'
        config.write_text(json.dumps(content))
        release.command('git','add','.',cwd=self.root)
        release.command('git','commit','--amend','--no-edit',cwd=self.root)
        data['commit']=release.command('git','rev-parse','HEAD',cwd=self.root)
        release.command('git','branch','-f','release-candidate',data['commit'],cwd=self.root)
        release.command('git','bundle','create',str(self.candidate/'candidate.bundle'),'release-candidate',cwd=self.root)
        data['bundle_sha256']=hashlib.sha256((self.candidate/'candidate.bundle').read_bytes()).hexdigest()
        (self.candidate/'metadata.json').write_text(json.dumps(data))
        release.command('git','checkout','--detach',self.initial,cwd=self.root)
        with self.assertRaisesRegex(ValueError,'deterministic version changes'):
            release.prepare(self.root,self.candidate,'patch')

    def test_draft_upload_failure_and_retry(self):
        import json, subprocess
        from unittest.mock import patch
        data=self.prepare()
        assets=self.base/'assets'; assets.mkdir()
        for platform in ['macOS-universal','Windows-x64']:
            (assets/f'Buz-It-v0.1.1-{platform}.zip').write_bytes(b'archive-fixture')
        real=release.command
        state={'draft':None,'uploaded':{},'fail_upload':True}
        def gh(*args,cwd=None):
            if args[0]!='gh': return real(*args,cwd=cwd)
            if args[1]=='api': return json.dumps([[state['draft']] if state['draft'] else []])
            operation=args[2]
            if operation=='create':
                state['draft']={'draft':True,'tag_name':'v0.1.1','body':args[args.index('--notes')+1]}
            elif operation=='upload':
                if state['fail_upload']:
                    state['fail_upload']=False
                    raise subprocess.CalledProcessError(1,args)
                for name in args[4:]:
                    if name.startswith('--'): continue
                    p=pathlib.Path(name); state['uploaded'][p.name]=p.stat().st_size
            elif operation=='view': return json.dumps({'assets':[{'name':n,'size':s} for n,s in state['uploaded'].items()]})
            elif operation=='edit': state['draft']['draft']=False
            else: raise AssertionError(args)
            return ''
        with patch.object(release,'command',side_effect=gh):
            with self.assertRaises(subprocess.CalledProcessError):
                release.publish(self.root,self.candidate,assets)
            self.assertTrue(state['draft']['draft'])
            release.publish(self.root,self.candidate,assets,draft_only=True)
            self.assertTrue(state['draft']['draft'])
            self.assertEqual(len(state['uploaded']),3)
            release.publish(self.root,self.candidate,assets)
            self.assertFalse(state['draft']['draft'])
            with self.assertRaisesRegex(ValueError,'already published'):
                release.publish(self.root,self.candidate,assets)
        self.assertEqual(release.remote_ref(self.root,'refs/tags/v0.1.1'),data['commit'])

    def test_candidate_metadata_rejects_extra_outputs(self):
        import json
        data=self.prepare()
        data['extra']='\ncommit=' + self.initial
        (self.candidate/'metadata.json').write_text(json.dumps(data))
        with self.assertRaisesRegex(ValueError,'metadata fields'):
            release.load_candidate(self.candidate)

class WindowsTextTests(unittest.TestCase):
    def test_candidate_reuse_with_windows_newlines_and_autocrlf(self):
        import os
        from unittest.mock import patch
        original=pathlib.Path.write_text
        def windows_write(path, data, *args, **kwargs):
            if kwargs.get('newline') is None: kwargs['newline']='\r\n'
            return original(path, data, *args, **kwargs)
        with patch.dict(os.environ, {'GIT_CONFIG_COUNT':'1', 'GIT_CONFIG_KEY_0':'core.autocrlf', 'GIT_CONFIG_VALUE_0':'true'}), patch.object(pathlib.Path,'write_text',windows_write):
            result=unittest.TestResult()
            GitReleaseTests('test_candidate_is_unpublished_and_reused_exactly').run(result)
        self.assertEqual(result.errors,[])
        self.assertEqual(result.failures,[])
