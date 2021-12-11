import Button from './Button';
import Ping from './Ping';
import type { Connection } from '../types/connection';

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
        <Ping ping={connection.bulkPing} />
      </td>
      <td>
        <Ping ping={connection.unorderedPing} />
      </td>
      <td>
        <Button onClick={handleKick}>Kick</Button>
      </td>
    </tr>
  );
};

export default TableRow;
