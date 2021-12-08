import Button from './Button';
import Ping from './Ping';

type TableRowProps = {
  connection: Connection;
};

const TableRow = ({ connection }: TableRowProps) => {
  function handleKick() {
    alert('A user was kicked');
  }

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
        <Button onClick={handleKick}>Kick</Button>
      </td>
    </tr>
  );
};

export default TableRow;
