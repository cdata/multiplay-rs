type TableRowProps = {
  connection: Connection;
};

const TableRow = ({ connection }: TableRowProps) => {
  return (
    <tr>
      <td>{connection.id}</td>
      <td>
        {connection.bulkConnection === false
          ? 'Disconnected'
          : connection.bulkConnection + 'ms'}
      </td>
      <td>
        {connection.unorderedConnection === false
          ? 'Disconnected'
          : connection.unorderedConnection + 'ms'}
      </td>
      <td>
        <button>Kick</button>
      </td>
    </tr>
  );
};

export default TableRow;
