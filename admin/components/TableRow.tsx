import Ping from './Ping';

type TableRowProps = {
  connection: Connection;
};

const TableRow = ({ connection }: TableRowProps) => {
  return (
    <tr>
      <td>{connection.id}</td>
      <td>
        <Ping ping={connection.bulkConnection} />
      </td>
      <td>
        <Ping ping={connection.unorderedConnection} />
      </td>
      <td>
        <button>Kick</button>
      </td>
    </tr>
  );
};

export default TableRow;
