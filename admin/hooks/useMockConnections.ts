import { useState, useEffect } from 'react';

const initialConnections: Connection[] = [
  { id: 'abc123xyz', bulkConnection: 20, unorderedConnection: 100 },
  { id: '5678asdf0', bulkConnection: false, unorderedConnection: 999 },
  { id: '321wsdf88', bulkConnection: 50, unorderedConnection: false },
];

export default function useMockConnections() {
  const [connections, setConnections] = useState(initialConnections);

  useEffect(() => {
    const interval = setInterval(() => {
      const newConnections = [...connections];
      newConnections.forEach((connection) => {
        connection.bulkConnection = Math.floor(Math.random() * 100);
        if (connection.bulkConnection < 10) {
          connection.bulkConnection = false;
        }

        connection.unorderedConnection = Math.floor(Math.random() * 100 * 10);
        if (connection.unorderedConnection > 700) {
          connection.unorderedConnection = false;
        }
      });
      setConnections(newConnections);
    }, 1000);
    return () => clearInterval(interval);
  }, []);

  return connections;
}
