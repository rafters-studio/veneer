// Compile huttspawn's Tailwind for exactly the classes the samples use, in a
// scratch directory. Reads the site; never writes there.
//
//   node tools/compile-css.mjs <scratch dir with tailwindcss@4.3.3 and @tailwindcss/cli@4.3.3 installed> <classes.txt>
//
// The input is the site's own: .rafters/output/rafters.css (the token registry's
// output, with its @theme, :root and .dark blocks) and the @layer base and @theme
// tail of src/styles/globals.css. Three edits, all of them removals:
//   - the @fontsource import is dropped (a web font an email cannot load);
//   - `@import "tailwindcss"` gets `source(none)`, and `@source` names the class
//     list, so the output holds exactly the samples' classes and not the site's;
//   - globals.css's site-only imports (shapes, card, panel) and its @font-face are not carried.
import { readFileSync, writeFileSync, copyFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { join, resolve } from 'node:path';

const SITE = '/Volumes/store/projects/huttspawn/site';
const [scratch, classes] = process.argv.slice(2).map((p) => resolve(p));

let rafters = readFileSync(join(SITE, '.rafters/output/rafters.css'), 'utf8');
rafters = rafters
  .replace('@import "@fontsource-variable/noto-sans";\n', '')
  .replace('@import "tailwindcss";', '@import "tailwindcss" source(none);');
writeFileSync(join(scratch, 'rafters.css'), rafters);

const globals = readFileSync(join(SITE, 'src/styles/globals.css'), 'utf8');
const tail = globals.slice(globals.indexOf('@layer base'));
copyFileSync(classes, join(scratch, 'classes.txt'));
writeFileSync(join(scratch, 'input.css'), `@import "./rafters.css";\n@source "./classes.txt";\n${tail}`);

execFileSync(join(scratch, 'node_modules/.bin/tailwindcss'), ['-i', 'input.css', '-o', 'compiled.css'], { cwd: scratch, stdio: 'inherit' });
copyFileSync(join(scratch, 'compiled.css'), 'css/huttspawn.compiled.css');
