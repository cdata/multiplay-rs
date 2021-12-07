import { useState, useEffect } from 'react';

export default function useConnections() {
  const [connections, setConnections] = useState(null);

  useEffect(() => {
    async function fetchConnections() {
      // Fetch connections.
      // If there is an error, just log it for now.
      try {
        const response = await fetch('/api/connections');
        const json = await response.json();
        setConnections(json);
      } catch (error) {
        console.error(error);
      }
    }

    fetchConnections();
  });

  return connections;
}
