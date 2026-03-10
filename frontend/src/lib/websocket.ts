import type { WsEvent } from './types';

type EventHandler = (event: WsEvent) => void;
type StatusHandler = (status: 'connected' | 'disconnected' | 'reconnecting', attempt: number) => void;

export class EventWebSocket {
  private ws: WebSocket | null = null;
  private handlers: EventHandler[] = [];
  private statusHandlers: StatusHandler[] = [];
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private attempt = 0;
  private stopped = false;

  connect() {
    this.stopped = false;
    this._connect();
  }

  private _connect() {
    const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
    const url = `${protocol}//${location.host}/ws/events`;
    this.ws = new WebSocket(url);

    this.ws.onopen = () => {
      this.attempt = 0;
      this._emitStatus('connected', 0);
    };

    this.ws.onmessage = (e) => {
      try {
        const event: WsEvent = JSON.parse(e.data);
        this.handlers.forEach((h) => h(event));
      } catch { /* ignore */ }
    };

    this.ws.onclose = () => {
      if (this.stopped) return;
      this.attempt++;
      const delay = Math.min(1000 * Math.pow(2, this.attempt - 1), 30000);
      this._emitStatus('reconnecting', this.attempt);
      this.reconnectTimer = setTimeout(() => this._connect(), delay);
    };

    this.ws.onerror = () => { this.ws?.close(); };
  }

  onEvent(handler: EventHandler) { this.handlers.push(handler); }
  onStatus(handler: StatusHandler) { this.statusHandlers.push(handler); }

  private _emitStatus(status: 'connected' | 'disconnected' | 'reconnecting', attempt: number) {
    this.statusHandlers.forEach((h) => h(status, attempt));
  }

  disconnect() {
    this.stopped = true;
    if (this.reconnectTimer) clearTimeout(this.reconnectTimer);
    this._emitStatus('disconnected', 0);
    this.ws?.close();
  }
}

export class AudioWebSocket {
  private ws: WebSocket | null = null;
  private path: string;

  constructor(path = '/ws/audio') { this.path = path; }

  connect() {
    const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
    this.ws = new WebSocket(`${protocol}//${location.host}${this.path}`);
  }

  send(data: ArrayBuffer) {
    if (this.ws?.readyState === WebSocket.OPEN) this.ws.send(data);
  }

  disconnect() { this.ws?.close(); }
}

export class VideoWebSocket {
  private ws: WebSocket | null = null;

  connect() {
    const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
    this.ws = new WebSocket(`${protocol}//${location.host}/ws/video`);
  }

  send(data: ArrayBuffer) {
    if (this.ws?.readyState === WebSocket.OPEN) this.ws.send(data);
  }

  disconnect() { this.ws?.close(); }
}
