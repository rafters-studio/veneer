import { forwardRef, memo } from 'react';

export const Ref = forwardRef<HTMLDivElement, { pad?: number }>(function Ref({ pad = 2 }, ref) {
  return <div ref={ref} data-pad={pad} />;
});

export const Memo = memo(function Memo({ dense = false }: { dense?: boolean }) {
  return <div data-dense={dense} />;
});
