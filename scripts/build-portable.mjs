import { spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { resolve } from "node:path";

const root = fileURLToPath(new URL("../", import.meta.url));
const run = (command, args) => {
  const result = spawnSync(command, args, { cwd: root, stdio: "inherit" });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
};
const cli = resolve(root, "node_modules/@tauri-apps/cli/tauri.js");
const releaseVersion = process.env.RELEASE_VERSION;
if (releaseVersion !== undefined) {
  if (!/^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$/.test(releaseVersion)) {
    throw new Error("RELEASE_VERSION must be a stable semantic version");
  }
  const config = JSON.parse(readFileSync(resolve(root, "src-tauri/tauri.conf.json"), "utf8"));
  if (config.version !== releaseVersion) throw new Error("Tauri version does not match release");
}
const prefix = releaseVersion ? `Buz-It-v${releaseVersion}` : "Buz-It";
const output = resolve(root, "dist");
mkdirSync(output, { recursive: true });
if (process.platform === "darwin") {
  run(process.execPath, [cli, "build", "--target", "universal-apple-darwin", "--bundles", "app"]);
  if (releaseVersion) {
    const result = spawnSync("/usr/libexec/PlistBuddy", ["-c", "Print CFBundleShortVersionString",
      resolve(root, "target/universal-apple-darwin/release/bundle/macos/Buz It.app/Contents/Info.plist")], { encoding: "utf8" });
    if (result.status !== 0 || result.stdout.trim() !== releaseVersion) throw new Error("Mac app version does not match release");
  }
  run("ditto", ["-c", "-k", "--sequesterRsrc", "--keepParent",
    resolve(root, "target/universal-apple-darwin/release/bundle/macos/Buz It.app"),
    resolve(output, `${prefix}-macOS-universal.zip`)]);
} else if (process.platform === "win32") {
  run(process.execPath, [cli, "build", "--target", "x86_64-pc-windows-msvc", "--no-bundle"]);
  copyFileSync(resolve(root, "target/x86_64-pc-windows-msvc/release/buz-it.exe"),
    resolve(output, "Buz It.exe"));
  if (releaseVersion) {
    run("powershell.exe", ["-NoProfile", "-NonInteractive", "-Command",
      `$ErrorActionPreference = 'Stop'; if ((Get-Item -LiteralPath 'dist/Buz It.exe').VersionInfo.ProductVersion -ne '${releaseVersion}') { throw 'Windows app version does not match release' }`]);
  }
  run("powershell.exe", ["-NoProfile", "-NonInteractive", "-Command",
    `$ErrorActionPreference = 'Stop'; Compress-Archive -LiteralPath 'dist/Buz It.exe' -DestinationPath 'dist/${prefix}-Windows-x64.zip' -Force`]);
} else {
  console.error("Portable builds support macOS and Windows. Build on the target operating system.");
  process.exit(1);
}
console.log(`Portable app saved in ${output}`);
