import { ReactButton } from '@/components/ui/react-button';
import { Button as ThirdButton } from 'third-party-ui';

function LocalButton(props: { size?: 'a' | 'b' }) {
  return <button>{props.size}</button>;
}

// T4 other
const plain: number = 'x';

export function Page() {
  return (
    <main>
      {/* T1 rafters undeclared prop on forwardRef */}
      <ReactButton label="a" tone="x" />
      {/* T2 rafters bad value on forwardRef */}
      <ReactButton label="a" variant="huge" />
      {/* T3 rafters missing required prop on forwardRef */}
      <ReactButton />
      {/* T5 other the project's own component */}
      <LocalButton tone="x" />
      {/* T6 other a third-party component */}
      <ThirdButton tone="x" />
      {/* T7 other a plain element */}
      <div nope="1">{plain}</div>
    </main>
  );
}
