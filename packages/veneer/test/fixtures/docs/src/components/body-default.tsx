export function BodyDefault(props: { size?: string }) {
  const { size = 'lg' } = props;
  return <div data-size={size} />;
}
