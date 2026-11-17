import type { Page } from "@playwright/test";

export const CORS = {
  "access-control-allow-origin": "*",
  "access-control-allow-headers": "content-type",
  "access-control-allow-methods": "GET, POST, OPTIONS",
};

export const WELCOME = {
  version: 1,
  session_id: "rs-viewer-e2e",
  token: "e2e-token",
  expires_at_ms: Date.now() + 3_600_000,
  tier: "webrtc",
  codec: "h264",
  dtls_fingerprint: "sha-256 00:11",
  policy: {
    allow_input: true,
    allow_clipboard: false,
    allow_audio: false,
    max_bitrate_kbps: 4_000,
    session_ttl_ms: 3_600_000,
  },
};

/**
 * A peer connection double. The browser build used by Playwright may not ship an
 * H.264 decoder, and CI has no host binary; this double proves the page's
 * negotiation, HUD and input wiring. The real media path is covered by the
 * opt-in live spec below and by the Rust transport tests.
 */
export async function installPeerConnectionDouble(page: Page): Promise<void> {
  await page.addInitScript(() => {
    class FakeChannel {
      readyState = "open";
      onopen: (() => void) | null = null;
      onclose: (() => void) | null = null;

      send(data: string): void {
        const sink = (window as unknown as { __viewerInput: string[] }).__viewerInput;
        sink.push(data);
      }

      close(): void {
        this.readyState = "closed";
        this.onclose?.();
      }
    }

    class FakePeerConnection {
      iceGatheringState = "complete";
      connectionState = "new";
      localDescription: { type: string; sdp: string } | null = null;
      ontrack: ((event: unknown) => void) | null = null;
      ondatachannel: ((event: unknown) => void) | null = null;
      onconnectionstatechange: (() => void) | null = null;

      constructor(public config: unknown) {
        (window as unknown as { __viewerPeer: unknown }).__viewerPeer = this;
      }

      addEventListener(): void {}
      removeEventListener(): void {}

      async setRemoteDescription(description: unknown): Promise<void> {
        (this as unknown as { remoteDescription: unknown }).remoteDescription = description;
      }

      async createAnswer(): Promise<{ type: string; sdp: string }> {
        return { type: "answer", sdp: "v=0\r\nfake-answer" };
      }

      async setLocalDescription(description: { sdp: string }): Promise<void> {
        this.localDescription = { type: "answer", sdp: description.sdp };
        window.setTimeout(() => {
          this.connectionState = "connected";
          this.onconnectionstatechange?.();
          const channel = new FakeChannel();
          this.ondatachannel?.({ channel });
          this.ontrack?.({ streams: [new MediaStream()], track: { kind: "video" } });
        }, 10);
      }

      getStats(): Promise<Map<string, unknown>> {
        return Promise.resolve(
          new Map<string, unknown>([
            [
              "inbound",
              {
                type: "inbound-rtp",
                kind: "video",
                framesDecoded: 240,
                framesPerSecond: 60,
                bytesReceived: 250_000,
                frameWidth: 640,
                frameHeight: 360,
              },
            ],
            [
              "pair",
              { type: "candidate-pair", nominated: true, currentRoundTripTime: 0.018 },
            ],
          ]),
        );
      }

      close(): void {
        this.connectionState = "closed";
        (window as unknown as { __viewerClosed: boolean }).__viewerClosed = true;
      }
    }

    (window as unknown as { __viewerInput: string[] }).__viewerInput = [];
    (window as unknown as { __viewerClosed: boolean }).__viewerClosed = false;
    (window as unknown as { RTCPeerConnection: unknown }).RTCPeerConnection =
      FakePeerConnection;
  });
}

export async function stubSignaling(page: Page, code: string): Promise<void> {
  await page.route("**/signal", async (route) => {
    if (route.request().method() === "OPTIONS") {
      await route.fulfill({ status: 204, headers: CORS });
      return;
    }
    const message = route.request().postDataJSON() as {
      type: string;
      pairing_code?: string;
    };
    if (message.type === "hello" && message.pairing_code !== code) {
      await route.fulfill({
        status: 403,
        headers: { ...CORS, "content-type": "application/json" },
        body: JSON.stringify({ error: "invalid pairing code" }),
      });
      return;
    }
    if (message.type === "hello") {
      await route.fulfill({
        status: 200,
        headers: { ...CORS, "content-type": "application/json" },
        body: JSON.stringify({
          welcome: WELCOME,
          offer: JSON.stringify({ type: "offer", sdp: "v=0\r\nfake-offer" }),
        }),
      });
      return;
    }
    await route.fulfill({
      status: 200,
      headers: { ...CORS, "content-type": "application/json" },
      body: JSON.stringify({ ok: true }),
    });
  });
}

export async function stubHealth(page: Page, source: string): Promise<void> {
  await page.route("**/health", async (route) => {
    await route.fulfill({
      status: 200,
      headers: { ...CORS, "content-type": "application/json" },
      body: JSON.stringify({ ok: true, code: "K7QM-2H4P", source }),
    });
  });
}