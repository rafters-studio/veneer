import type { BehavedBehaviorProps } from './behaved.behavior';

export function Behaved({ mode = 'auto', note }: BehavedBehaviorProps & { note?: string }) {
  return <div data-mode={mode}>{note}</div>;
}
