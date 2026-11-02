export function Fallback({ label }: { label?: string }) {
  const text = label ?? 'none';
  return <span>{text}</span>;
}
