import { spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { resolve } from "node:path";

const root = fileURLToPath(new URL("../", import.meta.url));
const run = (command, args) => {
  const result = spawnSync(command, args, { cwd: root, stdio: "inherit" });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
};
const cli = resolve(root, "node_modules/@tauri-apps/cli/tauri.js");
const output = resolve(root, "dist");
mkdirSync(output, { recursive: true });
if (process.platform === "darwin") {
  run(process.execPath, [cli, "build", "--target", "universal-apple-darwin", "--bundles", "app"]);
  run("ditto", ["-c", "-k", "--sequesterRsrc", "--keepParent",
    resolve(root, "target/universal-apple-darwin/release/bundle/macos/Buz It.app"),
    resolve(output, "Buz-It-macOS-universal.zip")]);
} else if (process.platform === "win32") {
  run(process.execPath, [cli, "build", "--target", "x86_64-pc-windows-msvc", "--no-bundle"]);
  copyFileSync(resolve(root, "target/x86_64-pc-windows-msvc/release/quiz-buzzer.exe"),
    resolve(output, "Buz It.exe"));
  run("powershell.exe", ["-NoProfile", "-NonInteractive", "-Command",
    "Compress-Archive -LiteralPath 'dist/Buz It.exe' -DestinationPath 'dist/Buz-It-Windows-x64.zip' -Force"]);
} else {
  console.error("Portable builds support macOS and Windows. Build on the target operating system.");
  process.exit(1);
}
console.log(`Portable app saved in ${output}`);
