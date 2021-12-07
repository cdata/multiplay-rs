import type { NextPage } from 'next';
import Head from 'next/head';
import Table from '../components/Table';

const Connections: NextPage = () => {
  return (
    <div>
      <Head>
        <title>Connections</title>
        <meta name="description" content="Server connections" />
        <link rel="icon" href="/favicon.ico" />
      </Head>

      <main>
        <h1>Connections</h1>
        <Table />
      </main>
    </div>
  );
};

export default Connections;
