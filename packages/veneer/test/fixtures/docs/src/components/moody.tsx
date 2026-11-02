/**
 * A calm panel for quiet content.
 *
 * Second paragraph of the description.
 *
 * @cognitive-load 2/10
 * @mood calm
 * @usage-patterns
 * DO: Use for secondary content
 * NEVER: Nest panels
 * @example
 * ```tsx
 * <Moody>
 *   <p>Hi</p>
 * </Moody>
 * ```
 */
import type { ReactNode } from 'react';

export function Moody({ children }: { children?: ReactNode }) {
  return <section>{children}</section>;
}
