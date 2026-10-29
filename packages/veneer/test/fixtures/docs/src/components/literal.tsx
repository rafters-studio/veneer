const sizes = { sm: 1, xl: 2, md: 3 } as const;

export interface LiteralProps {
  /** Voice of the label. */
  tone?: 'quiet' | 'loud' | 'calm';
  level: 3 | 1 | 2;
  size?: keyof typeof sizes;
}

export function Literal({ tone, level, size }: LiteralProps) {
  return <span data-tone={tone} data-level={level} data-size={size} />;
}
