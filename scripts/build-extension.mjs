import { cp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
const source = join(root, 'extension', 'src');
const publicDir = join(root, 'extension', 'public');
const out = join(root, 'extension', 'build');
await rm(out, { recursive: true, force: true });
for (const target of ['chromium', 'firefox']) {
  const destination = join(out, target);
  await mkdir(destination, { recursive: true });
  await cp(source, destination, { recursive: true });
  await writeFile(join(destination, 'manifest.json'), await readFile(join(publicDir, `manifest.${target}.json`)));
  for (const unused of target === 'chromium' ? ['background.firefox.js'] : ['background.chromium.js']) await rm(join(destination, unused));
}
console.log(`Extension artifacts created in ${out}`);
