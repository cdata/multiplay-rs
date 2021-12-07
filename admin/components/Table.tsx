import useMockConnections from '../hooks/useMockConnections';
import TableRow from './TableRow';

const Table = () => {
  const connections = useMockConnections();

  return (
    <div className="table-wrapper">
      <div className="table-heading">
        <h2>All connections</h2>
      </div>
      <table className="table">
        <thead>
          <tr>
            <td>Session ID</td>
            <td>Bulk connection</td>
            <td>Unordered connection</td>
            <td>Kick?</td>
          </tr>
        </thead>
        <tbody>
          {connections.map((connection) => (
            <TableRow key={connection.id} connection={connection} />
          ))}
        </tbody>
      </table>
    </div>
  );
};

export default Table;
