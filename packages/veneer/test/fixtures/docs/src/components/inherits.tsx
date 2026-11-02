import type { ButtonHTMLAttributes } from 'react';

export function Inherits({ type = 'button', ...rest }: ButtonHTMLAttributes<HTMLButtonElement>) {
  return <button type={type} {...rest} />;
}
