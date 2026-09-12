"""Version and publish portable releases; requires Python 3.11+, Git, and gh."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tomllib
import tempfile

VERSION_FILES = ['Cargo.toml', 'Cargo.lock', 'src-tauri/tauri.conf.json']


def command(*args, cwd=None):
    return subprocess.check_output(args, cwd=cwd, text=True).strip()


def validate(version):
    if not re.fullmatch(r'(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)', version):
        raise ValueError(f'Invalid stable version: {version}')
    return version


def bump(version, kind):
    parts = list(map(int, validate(version).split('.')))
    if kind not in ('major', 'minor', 'patch'):
        raise ValueError(f'Invalid bump: {kind}')
    index = ('major', 'minor', 'patch').index(kind)
    parts[index] += 1
    parts[index + 1:] = [0] * (2 - index)
    return '.'.join(map(str, parts))


def read_version(root):
    return validate(tomllib.loads((root / 'Cargo.toml').read_text())['workspace']['package']['version'])


def workspace_names(root):
    manifest = tomllib.loads((root / 'Cargo.toml').read_text())
    return {tomllib.loads((root / member / 'Cargo.toml').read_text())['package']['name']
            for member in manifest['workspace']['members']}


def check_versions(root, expected=None):
    version = read_version(root)
    if expected is not None and version != validate(expected):
        raise ValueError('Workspace version does not match expected release version')
    if json.loads((root / 'src-tauri/tauri.conf.json').read_text())['version'] != version:
        raise ValueError('Tauri and workspace versions differ')
    packages = tomllib.loads((root / 'Cargo.lock').read_text())['package']
    for name in workspace_names(root):
        matches = [p for p in packages if p['name'] == name and 'source' not in p]
        if len(matches) != 1 or matches[0]['version'] != version:
            raise ValueError(f'Lockfile version differs for {name}')
    return version


def set_version(root, version):
    validate(version)
    check_versions(root)  # Validate everything before writing anything.
    manifest = (root / 'Cargo.toml').read_text()
    pattern = r'(\[workspace\.package\][\s\S]*?\nversion\s*=\s*")[^"]+("[^\n]*)'
    manifest, count = re.subn(pattern, lambda m: m[1] + version + m[2], manifest, count=1)
    if count != 1:
        raise ValueError('Workspace version field not found')
    config = json.loads((root / 'src-tauri/tauri.conf.json').read_text())
    config['version'] = version
    names = workspace_names(root)
    blocks = (root / 'Cargo.lock').read_text().split('[[package]]')
    for index, block in enumerate(blocks[1:], 1):
        package = tomllib.loads('[[package]]' + block)['package'][0]
        if package['name'] in names and 'source' not in package:
            blocks[index] = re.sub(r'(?m)^version = "[^"]+"', f'version = "{version}"', block, count=1)
    (root / 'Cargo.toml').write_text(manifest, encoding='utf-8', newline='\n')
    (root / 'Cargo.lock').write_text('[[package]]'.join(blocks), encoding='utf-8', newline='\n')
    (root / 'src-tauri/tauri.conf.json').write_text(json.dumps(config, indent=2) + '\n', encoding='utf-8', newline='\n')


def load_candidate(directory):
    data = json.loads((directory / 'metadata.json').read_text())
    fields = {'base', 'commit', 'version', 'bundle_sha256'}
    if not isinstance(data, dict) or set(data) != fields or any(not isinstance(value, str) for value in data.values()):
        raise ValueError('Invalid candidate metadata fields')
    validate(data['version'])
    if not re.fullmatch('[0-9a-f]{64}', data['bundle_sha256']):
        raise ValueError('Invalid bundle checksum')
    for field in ('base', 'commit'):
        if not re.fullmatch('[0-9a-f]{40}', data[field]):
            raise ValueError('Invalid candidate commit')
    if hashlib.sha256((directory / 'candidate.bundle').read_bytes()).hexdigest() != data['bundle_sha256']:
        raise ValueError('Candidate bundle checksum differs')
    return data


def prepare(root, directory, kind):
    directory.mkdir(parents=True, exist_ok=True)
    if (directory / 'metadata.json').exists():
        data = load_candidate(directory)
        verify_reused(root, directory, data, kind)
        emit(data, reused='true')
        return
    if command('git', 'status', '--porcelain', cwd=root):
        raise ValueError('Candidate preparation requires a clean checkout')
    base = command('git', 'rev-parse', 'HEAD', cwd=root)
    version = bump(check_versions(root), kind)
    set_version(root, version)
    command('git', 'add', '--', *VERSION_FILES, cwd=root)
    command('git', '-c', 'user.name=github-actions[bot]', '-c',
            'user.email=41898282+github-actions[bot]@users.noreply.github.com',
            'commit', '-m', f'chore: release v{version}', cwd=root)
    commit = command('git', 'rev-parse', 'HEAD', cwd=root)
    command('git', 'branch', 'release-candidate', commit, cwd=root)
    command('git', 'bundle', 'create', str(directory / 'candidate.bundle'), 'release-candidate', cwd=root)
    data = dict(base=base, commit=commit, version=version,
                bundle_sha256=hashlib.sha256((directory / 'candidate.bundle').read_bytes()).hexdigest())
    (directory / 'metadata.json').write_text(json.dumps(data, indent=2) + '\n')
    emit(data, reused='false')


def verify_reused(root, directory, data, kind):
    # On a full Actions rerun checkout still points to the original dispatched SHA.
    base = command('git', 'rev-parse', 'HEAD', cwd=root)
    if data['base'] != base or data['version'] != bump(read_version(root), kind):
        raise ValueError('Saved candidate does not match the dispatched source and bump')
    command('git', 'fetch', str(directory / 'candidate.bundle'), 'release-candidate', cwd=root)
    if command('git', 'rev-parse', data['commit'] + '^', cwd=root) != base:
        raise ValueError('Saved candidate has an unexpected parent')
    changed = set(command('git', 'diff', '--name-only', base, data['commit'], cwd=root).splitlines())
    if changed != set(VERSION_FILES):
        raise ValueError('Saved candidate contains unexpected changes')
    with tempfile.TemporaryDirectory() as temporary:
        expected = Path(temporary)
        members = tomllib.loads((root / 'Cargo.toml').read_text())['workspace']['members']
        for name in VERSION_FILES + [member + '/Cargo.toml' for member in members]:
            destination = expected / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(subprocess.check_output(['git', 'show', f'{base}:{name}'], cwd=root))
        set_version(expected, data['version'])
        for name in VERSION_FILES:
            actual = subprocess.check_output(['git', 'show', f'{data["commit"]}:{name}'], cwd=root)
            if actual != (expected / name).read_bytes():
                raise ValueError('Saved candidate differs from deterministic version changes')


def emit(data, **extra):
    output = '\n'.join(f'{key}={value}' for key, value in {**data, **extra}.items()) + '\n'
    if os.getenv('GITHUB_OUTPUT'):
        with open(os.environ['GITHUB_OUTPUT'], 'a') as stream:
            stream.write(output)
    print(output, end='')


def restore(root, directory, expected_commit=None, expected_version=None):
    data = load_candidate(directory)
    if expected_commit is not None and data['commit'] != expected_commit:
        raise ValueError('Candidate does not match trusted preparation commit')
    if expected_version is not None and data['version'] != expected_version:
        raise ValueError('Candidate does not match trusted preparation version')
    command('git', 'fetch', str(directory / 'candidate.bundle'), 'release-candidate', cwd=root)
    command('git', 'checkout', '--detach', data['commit'], cwd=root)
    check_versions(root, data['version'])
    if command('git', 'rev-parse', 'HEAD^', cwd=root) != data['base']:
        raise ValueError('Candidate is not a direct child of its base')
    changed = set(command('git', 'diff-tree', '--no-commit-id', '--name-only', '-r', 'HEAD', cwd=root).splitlines())
    if changed != set(VERSION_FILES):
        raise ValueError('Candidate contains unexpected changes')
    return data


def remote_ref(root, ref):
    result = command('git', 'ls-remote', 'origin', ref, cwd=root)
    return result.split()[0] if result else None


def push_candidate(root, data):
    tag = 'v' + data['version']
    existing = remote_ref(root, f'refs/tags/{tag}')
    if existing:
        if existing != data['commit']:
            raise ValueError('Existing tag points at a different commit; refusing to move it')
        return  # Resume a publication whose atomic push already succeeded.
    if remote_ref(root, 'refs/heads/main') != data['base']:
        raise ValueError('main advanced during packaging; start a new release run')
    if command('git', 'tag', '--list', tag, cwd=root):
        if command('git', 'rev-parse', f'refs/tags/{tag}', cwd=root) != data['commit']:
            raise ValueError('Local tag points at a different commit')
    else:
        command('git', 'tag', tag, data['commit'], cwd=root)
    command('git', 'push', '--atomic', 'origin', f'{data["commit"]}:refs/heads/main',
            f'refs/tags/{tag}:refs/tags/{tag}', cwd=root)


def publish(root, directory, assets, draft_only=False, expected_commit=None, expected_version=None):
    data = restore(root, directory, expected_commit, expected_version)
    tag = 'v' + data['version']
    names = [f'Buz-It-{tag}-{platform}.zip' for platform in ('macOS-universal', 'Windows-x64')]
    for name in names:
        if not (assets / name).is_file() or (assets / name).stat().st_size == 0:
            raise ValueError(f'Missing archive: {name}')
    checksum = assets / 'SHA256SUMS.txt'
    checksum.write_text(''.join(f'{hashlib.sha256((assets / name).read_bytes()).hexdigest()}  {name}\n' for name in names))
    push_candidate(root, data)
    # Enumerate releases so authentication/network failures cannot be mistaken for a missing release.
    pages = json.loads(command('gh', 'api', '--paginate', '--slurp', 'repos/{owner}/{repo}/releases', cwd=root))
    existing = next((r for page in pages for r in page if r['tag_name'] == tag), None)
    marker = f'<!-- buz-it-candidate:{data["commit"]} -->'
    if existing and not existing['draft']:
        raise ValueError('Release is already published; refusing to overwrite it')
    if existing and marker not in (existing['body'] or ''):
        raise ValueError('Draft belongs to another publication; refusing to overwrite it')
    if not existing:
        command('gh', 'release', 'create', tag, '--verify-tag', '--draft', '--title', f'Buz It {tag}',
                '--generate-notes', '--notes', marker, cwd=root)
    command('gh', 'release', 'upload', tag, *(str(assets / n) for n in names), str(checksum), '--clobber', cwd=root)
    uploaded = json.loads(command('gh', 'release', 'view', tag, '--json', 'assets', cwd=root))['assets']
    sizes = {a['name']: a['size'] for a in uploaded}
    for name in names + ['SHA256SUMS.txt']:
        if sizes.get(name) != (assets / name).stat().st_size:
            raise ValueError(f'Uploaded asset missing or incomplete: {name}')
    if not draft_only:
        command('gh', 'release', 'edit', tag, '--draft=false', '--latest', cwd=root)
    print(f'{tag}: ' + ('draft verified' if draft_only else 'published'))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('operation', choices=['prepare', 'restore', 'check', 'publish'])
    parser.add_argument('--bump', choices=['patch', 'minor', 'major'], default='patch')
    parser.add_argument('--candidate', type=Path, default=Path('candidate'))
    parser.add_argument('--assets', type=Path, default=Path('release-assets'))
    parser.add_argument('--expected')
    parser.add_argument('--expected-commit')
    parser.add_argument('--draft-only', action='store_true')
    args = parser.parse_args()
    root = Path.cwd()
    if os.getenv('GITHUB_ACTIONS') and args.operation in ('restore', 'publish') and not (args.expected_commit and args.expected):
        raise ValueError('Actions restoration requires trusted commit and version outputs')
    if args.operation == 'prepare': prepare(root, args.candidate.resolve(), args.bump)
    elif args.operation == 'restore': emit(restore(root, args.candidate.resolve(), args.expected_commit, args.expected))
    elif args.operation == 'check': print(check_versions(root, args.expected))
    else: publish(root, args.candidate.resolve(), args.assets.resolve(), args.draft_only, args.expected_commit, args.expected)


if __name__ == '__main__':
    main()
