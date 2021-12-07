type Ping = {
  ping: number | false;
};

export default function Ping({ ping }: Ping) {
  return <>{ping === false ? 'Disconnected' : ping + 'ms'}</>;
}
