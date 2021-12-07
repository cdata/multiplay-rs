import type { NextPage } from 'next';
import Head from 'next/head';
import Table from '../components/Table';
import styles from '../styles/Home.module.css';

// Mock data
const data: Connection[] = [
  { id: 'abc123xyz', bulkConnection: 20, unorderedConnection: 100 },
  { id: '5678asdf0', bulkConnection: false, unorderedConnection: 999 },
  { id: '321wsdf88', bulkConnection: 50, unorderedConnection: false },
];

const Connections: NextPage = () => {
  return (
    <div className={styles.container}>
      <Head>
        <title>Connections</title>
        <meta name="description" content="Server connections" />
        <link rel="icon" href="/favicon.ico" />
      </Head>

      <main className={styles.main}>
        <h1>Connections</h1>
        <Table connections={data} />
      </main>

      <footer className={styles.footer}>
        <p>footer</p>
      </footer>
    </div>
  );
};

export default Connections;
