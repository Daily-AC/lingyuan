import type { ServerMsg } from './types';

export type MsgHandler = (msg: ServerMsg) => void;

export interface WsHandle {
  close: () => void;
}

const RECONNECT_DELAY_MS = 2000;

export function connect(url: string, onMsg: MsgHandler): WsHandle {
  let closedByUser = false;
  let socket: WebSocket | null = null;
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

  const open = (): void => {
    console.log('[ws] connecting', url);
    const ws = new WebSocket(url);
    socket = ws;

    ws.addEventListener('open', () => {
      console.log('[ws] open', url);
    });

    ws.addEventListener('message', (ev: MessageEvent) => {
      if (typeof ev.data !== 'string') {
        return;
      }
      try {
        const parsed = JSON.parse(ev.data) as ServerMsg;
        onMsg(parsed);
      } catch (err) {
        console.error('[ws] bad json', err, ev.data);
      }
    });

    ws.addEventListener('error', (ev: Event) => {
      console.error('[ws] error', ev);
    });

    ws.addEventListener('close', (ev: CloseEvent) => {
      console.warn('[ws] close', ev.code, ev.reason);
      socket = null;
      if (closedByUser) {
        return;
      }
      reconnectTimer = setTimeout(open, RECONNECT_DELAY_MS);
    });
  };

  open();

  return {
    close(): void {
      closedByUser = true;
      if (reconnectTimer !== null) {
        clearTimeout(reconnectTimer);
        reconnectTimer = null;
      }
      if (socket !== null) {
        socket.close();
        socket = null;
      }
    },
  };
}
