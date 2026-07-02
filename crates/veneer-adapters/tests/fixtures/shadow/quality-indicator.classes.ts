/**
 * Quality indicator tint classes.
 *
 * The tint arrives as a prop (color={tint}), so the resolved class names
 * (text-quality-500, text-quality-600) never appear as source literals --
 * Tailwind tree-shakes them from compiled output. Fixture modeled on the
 * dynamic-composition case described in bullpen post 019f1f4d.
 */

export const qualityTintClass = (tint: string): string => `text-quality-${tint}`;

export const qualityBaseClasses = 'inline-flex items-center gap-1';
