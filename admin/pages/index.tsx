import type { NextPage } from 'next';
import Head from 'next/head';
import Table from '../components/Table';

// Mock data
const data: Connection[] = [
  { id: 'abc123xyz', bulkConnection: 20, unorderedConnection: 100 },
  { id: '5678asdf0', bulkConnection: false, unorderedConnection: 999 },
  { id: '321wsdf88', bulkConnection: 50, unorderedConnection: false },
];

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
        <Table connections={data} />
      </main>
    </div>
  );
};

export default Connections;
