export type ServerDataPingEvent = {
  sessionId: string;
  ping: number;
  transport: 'bulk' | 'unordered';
};

export type ServerDataConnectedEvent = {
  id: string;
};

export type ServerDataDisconnectedEvent = {
  id: string;
};

export default class ServerData extends EventTarget {
  #sessions = [];

  constructor() {
    super();

    this.#sessions.push(this.#makeSession());
    this.#dispatch('connected', this.#sessions[0]);

    this.#loop();
  }

  #makeSession = () => {
    return {
      id: Math.floor(Math.random() * 8999999) + 1000000,
    };
  };

  #loop = () => {
    requestAnimationFrame(() => {
      if (Math.random() > 0.95) {
        this.#emitData();
      }

      this.#loop();
    });
  };

  #emitData = () => {
    let dataType = Math.floor(Math.random() * 8);

    switch (dataType) {
      case 0:
      case 1:
      case 2:
      case 3:
      case 4:
        this.ping();
        break;
      case 5:
      case 6:
        this.connected();
        break;
      case 7:
        this.disconnected();
        break;
    }
  };

  #dispatch = (event, data) => {
    this.dispatchEvent(
      new CustomEvent(event, {
        detail: data,
      })
    );
  };

  connected() {
    if (this.#sessions.length > 7) {
      return;
    }
    const session = this.#makeSession();
    this.#sessions.push(session);
    this.#dispatch('connected', session);
  }

  disconnected() {
    if (this.#sessions.length === 0) {
      return;
    }
    const session = this.#sessions.splice(
      Math.floor(Math.random() * this.#sessions.length),
      1
    )[0];
    this.#dispatch('disconnected', session);
  }

  ping() {
    if (this.#sessions.length === 0) {
      return;
    }
    const session =
      this.#sessions[Math.floor(Math.random() * this.#sessions.length)];
    this.#dispatch('ping', {
      sessionId: session.id,
      ping: Math.floor(Math.random() * 200 + 8),
      transport: Math.random() > 0.5 ? 'bulk' : 'unordered',
    });
  }
}
