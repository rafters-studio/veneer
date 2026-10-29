export function Toggle(props: { on?: boolean }) {
  return <button aria-pressed={props.on} />;
}
