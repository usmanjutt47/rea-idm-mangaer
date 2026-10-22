import { cp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

const root = new URL('..', import.meta.url).pathname;
const source = join(root, 'extension', 'src');
const publicDir = join(root, 'extension', 'public');
const out = join(root, 'extension', 'build');
await rm(out, { recursive: true, force: true });
await mkdir(out, { recursive: true });
await cp(source, out, { recursive: true });
for (const target of ['chromium', 'firefox']) await writeFile(join(out, `manifest.${target}.json`), await readFile(join(publicDir, `manifest.${target}.json`)));
console.log(`Extension artifacts created in ${out}`);
