/**
 * Shared badge variant and size class definitions
 *
 * Imported by both badge.tsx (React) and badge.astro (Astro)
 * to ensure visual parity across framework targets.
 *
 * Fixture modeled verbatim on rafters
 * packages/ui/src/old/ui/badge.classes.ts.
 */

export const badgeVariantClasses: Record<string, string> = {
  default: 'bg-primary text-primary-foreground',
  primary: 'bg-primary text-primary-foreground',
  secondary: 'bg-secondary text-secondary-foreground',
  muted: 'bg-muted text-muted-foreground',
  outline: 'bg-transparent border border-input text-foreground',
  ghost: 'hover:bg-muted hover:text-muted-foreground',
};

export const badgeSizeClasses: Record<string, string> = {
  sm: 'px-2 py-0.5 text-label-small',
  default: 'px-2.5 py-0.5 text-label-small',
  lg: 'px-3 py-1 text-label-medium',
};

export const badgeBaseClasses =
  'inline-flex items-center justify-center rounded-full transition-colors duration-150 motion-reduce:transition-none';
