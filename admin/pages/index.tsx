import type { NextPage } from 'next';
import Head from 'next/head';
import SideBar from '../components/SideBar';
import Table from '../components/Table';

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
