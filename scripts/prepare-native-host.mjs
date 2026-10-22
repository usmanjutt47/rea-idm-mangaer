import { copyFile, mkdir, stat } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { join } from 'node:path';

const root = fileURLToPath(new URL('..', import.meta.url));
const tauriRoot = join(root, 'src-tauri');
execFileSync('cargo', ['build', '--release', '--manifest-path', join(tauriRoot, 'Cargo.toml'), '--bin', 'rea-idm-native-host'], { stdio: 'inherit' });
const source = join(tauriRoot, 'target', 'release', 'rea-idm-native-host.exe');
await stat(source);
const destinationDir = join(tauriRoot, 'binaries');
await mkdir(destinationDir, { recursive: true });
await copyFile(source, join(destinationDir, 'rea-idm-native-host.exe'));
console.log('Prepared native browser host as an installer resource.');
