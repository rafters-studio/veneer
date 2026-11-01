import { forwardRef } from 'react';

export interface ReactButtonProps {
  variant?: 'default' | 'ghost';
  label: string;
}

export const ReactButton = forwardRef<HTMLButtonElement, ReactButtonProps>(function ReactButton({ variant, label }, ref) {
  return <button ref={ref} data-variant={variant}>{label}</button>;
});
