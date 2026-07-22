/**
 * Primary action trigger.
 *
 * @cognitive-load 3/10 - Simple action trigger
 * @usage-patterns
 * DO: One primary action per section
 * NEVER: Multiple primary buttons competing for attention
 */
export interface ButtonProps {
  variant?: 'default' | 'secondary';
}

const variantClasses = {
  default: 'bg-primary text-primary-foreground',
  secondary: 'bg-secondary text-secondary-foreground',
};

export function Button() {
  return <button />;
}
