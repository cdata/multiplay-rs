import type { NextPage } from 'next';
import Head from 'next/head';
import dynamic from 'next/dynamic';

// Table uses the useMockServerData hook, which relies on
// CustomEvent and requestAnimationFrame. Neither of which are
// available on the server, so we need to tell Next to only load
// this component on the client.
const Table = dynamic(() => import('../components/Table'), { ssr: false });

const Connections: NextPage = () => {
  return (
    <>
      <Head>
        <title>Connections</title>
        <meta name="description" content="Server connections" />
        <link rel="icon" href="/favicon.ico" />
      </Head>

      <h1>Connections</h1>
      <Table />
    </>
  );
};

export default Connections;
