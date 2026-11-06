// The transactional and newsletter content through react-email 2 (@react-email/render
// 2.2.0, @react-email/tailwind 2.0.7), with the same rafters class strings on each
// element. react-email's Tailwind cannot read a CSS-first @theme (its stylesheet
// loader serves only Tailwind's own files), so the huttspawn values our transform
// resolved are passed in its JS config instead.
//
//   node tools/react-email.mjs <dir holding node_modules/@react-email/components>
import { createRequire } from 'node:module';
import { writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

const require = createRequire(join(resolve(process.argv[2]), 'package.json'));
const React = require('react');
const { Html, Head, Body, Container, Section, Row, Column, Heading, Text, Img, Button, Hr, Tailwind } = require('@react-email/components');
const { render } = require('@react-email/render');
const h = React.createElement;

// The light values out/*.html inlined (light only: there is no dark source to give it).
const config = {
  theme: {
    extend: {
      colors: {
        background: '#e7f1f1', foreground: '#000101', primary: '#3d5c5c', 'primary-foreground': '#e7f1f1',
        'primary-hover': '#3d5c5c', 'primary-active': '#3d5c5c', border: '#a8c9c9', 'muted-foreground': '#052525',
        'primary-ring': '#3d5c5c',
      },
      fontFamily: { sans: ['Inter', 'ui-sans-serif', 'system-ui', 'sans-serif'] },
      fontSize: {
        '4xl': ['39.808px', { lineHeight: '1.25' }], '3xl': ['33.184px', { lineHeight: '1.3' }],
        sm: ['13.328px', { lineHeight: '1.5', letterSpacing: '.015em' }],
        'label-large': ['16px', { lineHeight: '1.5', fontWeight: '500' }],
      },
      borderRadius: { md: '.192px', lg: '.2304px' },
    },
  },
};

// Class strings exactly as the rafters components emit them (src/rafter.rs).
const H1 = 'scroll-m-20 text-4xl font-bold tracking-tight text-foreground @lg:text-5xl';
const H2 = 'scroll-m-20 text-3xl font-semibold tracking-tight text-foreground';
const P = 'leading-7 text-foreground';
const MUTED = 'text-sm text-muted-foreground';
const button = (size) =>
  'inline-grid grid-flow-col items-center justify-center gap-2 rounded-md text-label-large ts-label-large cursor-pointer ' +
  'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 transition-[color,background-color,border-color,box-shadow,scale] duration-fast ease-standard ' +
  'scale-100 extent-press active:scale-(--rafters-consumed-extent) active:duration-micro active:ease-spring-snappy disabled:opacity-50 disabled:cursor-not-allowed ' +
  'aria-disabled:opacity-50 aria-disabled:cursor-not-allowed aria-busy:cursor-progress bg-primary text-primary-foreground hover:bg-primary-hover active:bg-primary-active ' +
  'focus-visible:ring-2 focus-visible:ring-primary-ring ' +
  (size === 'lg' ? 'h-12 px-6 text-label-large ts-label-large' : 'h-11 px-4 py-2.5 @md:h-10 @md:py-2');
const SEP = 'shrink-0 bg-border h-px w-full';

const shell = (...children) =>
  h(Tailwind, { config }, h(Html, { lang: 'en' }, h(Head), h(Body, { className: 'bg-background text-foreground font-sans' },
    h(Container, { className: 'max-w-2xl mx-auto px-4' }, ...children))));

const emails = {
  transactional: shell(
    h(Heading, { as: 'h1', className: H1 }, 'Confirm your email'),
    h(Text, { className: P }, 'Someone, hopefully you, made a Huttspawn account with this address. Confirm it and the account is yours. The link works for 24 hours.'),
    h(Button, { href: 'https://huttspawn.com/verify?token=example', className: button('lg') }, 'Confirm email'),
    h(Hr, { className: SEP }),
    h(Text, { className: MUTED }, 'If you did not sign up, ignore this email and nothing happens.'),
    h(Text, { className: MUTED }, 'Huttspawn, a Star Wars: The Old Republic guild site.'),
  ),
  newsletter: shell(
    h(Heading, { as: 'h1', className: H1 }, 'The Outer Rim, this month'),
    h(Text, { className: P }, 'Three new discipline guides landed, the GSF ladder reset, and the bounty board is open again. Here is what changed and where to start.'),
    h(Img, { src: 'https://huttspawn.com/images/bounty-hunter.png', alt: 'A bounty hunter in Mandalorian armour', width: 600, className: 'block w-full h-auto rounded-lg' }),
    h(Button, { href: 'https://huttspawn.com/disciplines', className: button('default') }, 'Read the new guides'),
    h(Hr, { className: SEP }),
    h(Section, null, h(Row, null,
      h(Column, { className: 'w-1/2 align-top pr-3' }, h(Heading, { as: 'h2', className: H2 }, 'GSF ladder'), h(Text, { className: P }, 'Season nine started on Tuesday. Ratings reset to 1500 and the first match night is Friday.')),
      h(Column, { className: 'w-1/2 align-top pl-3' }, h(Heading, { as: 'h2', className: H2 }, 'Bounty board'), h(Text, { className: P }, 'Eleven contracts are open. The oldest pays double and closes at the end of the week.')),
    )),
  ),
};

for (const [name, el] of Object.entries(emails)) {
  let html;
  try {
    html = await render(el);
  } catch (e) {
    console.log(`${name}: react-email threw: ${e.message.split('\n')[0]}`);
    continue;
  }
  writeFileSync(`out/${name}.react-email.html`, html);
  const times = [];
  for (let i = 0; i < 50; i++) {
    const t = performance.now();
    await render(el);
    times.push(performance.now() - t);
  }
  times.sort((a, b) => a - b);
  const style = (html.match(/<style>([\s\S]*?)<\/style>/) || [])[1] || '';
  console.log(`${name}: ${html.length} bytes, <style> ${style.length} bytes, render median ${times[25].toFixed(2)} ms (warm, 50 runs), var( ${(html.match(/var\(/g) || []).length}, oklch ${(html.match(/oklch/g) || []).length}`);
}
