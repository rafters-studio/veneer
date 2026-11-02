interface TextProps {
  kind?: 'text';
  label: string;
}

interface IconProps {
  kind: 'icon';
  icon: string;
}

export function Branches(props: TextProps | IconProps) {
  return <span>{'label' in props ? props.label : props.icon}</span>;
}
