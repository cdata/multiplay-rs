type Ping = {
  ping: number;
};

export default function Ping({ ping }: Ping) {
  let pingStatus = '';
  if (ping === 0) {
    pingStatus = 'disconnected';
  } else if (ping < 100) {
    pingStatus = 'good';
  } else if (ping > 100 && ping < 300) {
    pingStatus = 'ok';
  } else if (ping > 300) {
    pingStatus = 'bad';
  }
  return (
    <span className={`ping ping--${pingStatus}`}>
      {pingStatus === 'disconnected' ? 'Disconnected' : ping + 'ms'}
    </span>
  );
}
