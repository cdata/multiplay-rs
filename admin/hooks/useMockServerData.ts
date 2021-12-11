import type { Connection } from '../types/connection';
import ServerData from '../lib/serverData';
import type {
  ServerDataPingEvent,
  ServerDataConnectedEvent,
  ServerDataDisconnectedEvent,
} from '../lib/serverData';
import { useState, useEffect } from 'react';

export default function useMockServerData() {
  const [connections, setConnections] = useState<Connection[]>([]);
  const [serverData, setServerData] = useState<ServerData>(new ServerData());

  useEffect(() => {
    function handlePing(event) {
      const { sessionId, ping, transport } = (<any>event)
        .detail as ServerDataPingEvent;

      setConnections((connections) =>
        connections.map((connection) => {
          if (connection.id === sessionId) {
            return {
              ...connection,
              bulkPing: transport === 'bulk' ? ping : connection.bulkPing,
              unorderedPing:
                transport === 'unordered' ? ping : connection.unorderedPing,
            };
          }
          return connection;
        })
      );
    }

    function handleConnected(event) {
      const { id } = (<any>event).detail as ServerDataConnectedEvent;
      setConnections((connections) => [
        ...connections,
        { id, bulkPing: 0, unorderedPing: 0 },
      ]);
    }

    function handleDisconnected(event) {
      const { id } = (<any>event).detail as ServerDataDisconnectedEvent;
      setConnections((connections) =>
        connections.filter((connection) => connection.id !== id)
      );
    }

    serverData.addEventListener('ping', handlePing);
    serverData.addEventListener('connected', handleConnected);
    serverData.addEventListener('disconnected', handleDisconnected);
  }, []);

  return connections;
}
