declare module "@aether/design-system/water.js" {
  export interface WaterOptions {
    tone?: "lagoon" | "ocean" | "deep";
    speed?: "calm" | "flowing" | "rapid";
    intensity?: "subtle" | "balanced" | "lively";
    effects?: boolean;
    drips?: boolean;
    clickRipples?: boolean;
    caustics?: boolean;
    scope?: Element | null;
  }

  export class WaterSurface {
    constructor(canvas: HTMLCanvasElement | null, options?: WaterOptions);
    configure(patch?: WaterOptions): void;
    setTone(tone: NonNullable<WaterOptions["tone"]>): void;
    setSpeed(speed: NonNullable<WaterOptions["speed"]>): void;
    setIntensity(intensity: NonNullable<WaterOptions["intensity"]>): void;
    setEnabled(enabled: boolean): void;
    destroy(): void;
    resize(): void;
    start(): void;
    stop(): void;
  }

  export default WaterSurface;
}
