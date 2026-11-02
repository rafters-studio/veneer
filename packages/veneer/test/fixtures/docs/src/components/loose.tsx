export function Loose(props: { data: any; name?: string }) {
  return <pre>{String(props.data)}{props.name}</pre>;
}
