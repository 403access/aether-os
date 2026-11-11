/* ==========================================================================
   Data Cloud Compare — water surface
   --------------------------------------------------------------------------
   The interactive half of the water theme: slow caustic light, cursor drips,
   and click ripples. This is a dependency-free ES module distilled from
   app/src/web/lib/water.ts.

   const surface = new WaterSurface(canvas, {
     tone: "ocean",          // lagoon | ocean | deep
     speed: "flowing",       // calm | flowing | rapid
     intensity: "subtle",    // subtle | balanced | lively
     effects: true,
     drips: true,
     clickRipples: true,
     caustics: true,
   });

   Use `scope` to confine pointer events to a smaller element (the ripple
   pond in the showcase). Without it, the surface listens on `window`.
   ========================================================================== */

export const WATER_TONES = {
  lagoon: "85, 240, 224",
  ocean: "56, 214, 255",
  deep: "72, 183, 255",
};

export const WATER_INTENSITY = {
  subtle: { minDist: 32, throttle: 100, alpha: 0.16, grow: 9, clickAlpha: 0.26 },
  balanced: { minDist: 22, throttle: 74, alpha: 0.22, grow: 13, clickAlpha: 0.32 },
  lively: { minDist: 14, throttle: 50, alpha: 0.28, grow: 17, clickAlpha: 0.38 },
};

export const WATER_SPEEDS = { calm: 0.65, flowing: 1, rapid: 1.6 };

export const WATER_DEFAULTS = {
  tone: "ocean",
  speed: "flowing",
  intensity: "subtle",
  effects: true,
  drips: true,
  clickRipples: true,
  caustics: true,
};

const MAX_PARTICLES = 90;

function prefersReducedMotion() {
  return (
    typeof window !== "undefined" &&
    window.matchMedia?.("(prefers-reduced-motion: reduce)").matches
  );
}

export class WaterSurface {
  constructor(canvas, options = {}) {
    if (!canvas) throw new Error("WaterSurface needs a canvas element");
    this.canvas = canvas;
    this.ctx = canvas.getContext ? canvas.getContext("2d") : null;
    this.settings = { ...WATER_DEFAULTS, ...options };
    this.scope = options.scope || (typeof window !== "undefined" ? window : null);
    this.enabled = options.enabled !== false;
    this.running = false;
    this.raf = 0;
    this.lastFrame = 0;
    this.w = 0;
    this.h = 0;
    this.particles = [];
    this.pointer = null;
    this.lastSpawn = 0;
    this.lastX = 0;
    this.lastY = 0;

    this.onResize = () => this.resize();
    this.onPointerMove = (event) => this.pointerMove(event);
    this.onPointerDown = (event) => this.pointerDown(event);
    this.onVisibility = () => {
      if (document.hidden) this.stop();
      else if (this.enabled && !prefersReducedMotion()) this.start();
    };

    if (this.scope) {
      this.scope.addEventListener("pointermove", this.onPointerMove, { passive: true });
      this.scope.addEventListener("pointerdown", this.onPointerDown, { passive: true });
    }
    window.addEventListener("resize", this.onResize, { passive: true });
    document.addEventListener("visibilitychange", this.onVisibility);

    if (typeof ResizeObserver !== "undefined") {
      this.resizeObserver = new ResizeObserver(() => this.resize());
      this.resizeObserver.observe(canvas);
    }

    this.resize();
    if (this.enabled && !prefersReducedMotion()) this.start();
  }

  configure(patch = {}) {
    this.settings = { ...this.settings, ...patch };
    if ("enabled" in patch) this.enabled = Boolean(patch.enabled);
    if (this.enabled && !prefersReducedMotion()) this.start();
    else this.stop();
    return this;
  }

  setTone(tone) {
    if (WATER_TONES[tone]) this.settings.tone = tone;
    return this;
  }

  setSpeed(speed) {
    if (WATER_SPEEDS[speed]) this.settings.speed = speed;
    return this;
  }

  setIntensity(intensity) {
    if (WATER_INTENSITY[intensity]) this.settings.intensity = intensity;
    return this;
  }

  setEnabled(enabled) {
    this.enabled = Boolean(enabled);
    if (this.enabled && !prefersReducedMotion()) this.start();
    else this.stop();
    return this;
  }

  destroy() {
    this.stop();
    if (this.scope) {
      this.scope.removeEventListener("pointermove", this.onPointerMove);
      this.scope.removeEventListener("pointerdown", this.onPointerDown);
    }
    window.removeEventListener("resize", this.onResize);
    document.removeEventListener("visibilitychange", this.onVisibility);
    this.resizeObserver?.disconnect();
    this.ctx?.clearRect(0, 0, this.w, this.h);
  }

  resize() {
    if (!this.ctx) return;
    const dpr = Math.min(window.devicePixelRatio || 1, 1.5);
    const rect = this.canvas.getBoundingClientRect();
    const width = rect.width || window.innerWidth;
    const height = rect.height || window.innerHeight;
    this.w = width;
    this.h = height;
    this.canvas.width = Math.max(1, Math.round(width * dpr));
    this.canvas.height = Math.max(1, Math.round(height * dpr));
    this.ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  }

  start() {
    if (this.running || !this.ctx || prefersReducedMotion()) return;
    this.resize();
    this.lastFrame = performance.now();
    this.running = true;
    this.raf = requestAnimationFrame(this.frame);
  }

  stop() {
    this.running = false;
    if (this.raf) cancelAnimationFrame(this.raf);
    this.raf = 0;
    this.particles = [];
    this.pointer = null;
    this.ctx?.clearRect(0, 0, this.w, this.h);
  }

  point(event) {
    const rect = this.canvas.getBoundingClientRect();
    return {
      x: event.clientX - rect.left,
      y: event.clientY - rect.top,
      at: performance.now(),
    };
  }

  pointerMove(event) {
    if (!this.enabled) return;
    this.pointer = this.point(event);
    if (!this.settings.effects || !this.settings.drips) return;
    const cfg = WATER_INTENSITY[this.settings.intensity] || WATER_INTENSITY.subtle;
    const now = performance.now();
    if (now - this.lastSpawn < cfg.throttle) return;
    const point = this.pointer;
    if (Math.hypot(point.x - this.lastX, point.y - this.lastY) < cfg.minDist) return;
    this.lastSpawn = now;
    this.lastX = point.x;
    this.lastY = point.y;
    this.spawn(point.x, point.y, "drip");
  }

  pointerDown(event) {
    if (!this.enabled || !this.settings.effects || !this.settings.clickRipples) return;
    const point = this.point(event);
    this.pointer = point;
    this.spawn(point.x, point.y, "click");
  }

  spawn(x, y, kind) {
    const cfg = WATER_INTENSITY[this.settings.intensity] || WATER_INTENSITY.subtle;
    const click = kind === "click";
    if (this.particles.length > MAX_PARTICLES) this.particles.shift();
    this.particles.push({
      x,
      y,
      age: 0,
      life: click ? 1500 : 950,
      start: click ? 2 : 1,
      grow: click ? 38 : cfg.grow,
      alpha: click ? cfg.clickAlpha : cfg.alpha,
      width: click ? 1.7 : 1.05,
      wobble: Math.random() * Math.PI * 2,
    });
  }

  drawCaustics(now) {
    if (!this.ctx || !this.settings.caustics) return;
    const rgb = WATER_TONES[this.settings.tone] || WATER_TONES.ocean;
    const speed = WATER_SPEEDS[this.settings.speed] || 1;
    const t = now * 0.001 * speed;
    const ctx = this.ctx;
    ctx.save();
    ctx.globalCompositeOperation = "lighter";
    for (let i = 0; i < 5; i += 1) {
      const cx = this.w * (0.5 + 0.44 * Math.sin(t * 0.11 * (i + 1) + i * 1.7));
      const cy = this.h * (0.5 + 0.42 * Math.cos(t * 0.14 * (i + 1) + i * 0.83));
      const radius = Math.max(this.w, this.h) * (0.2 + 0.055 * i);
      const gradient = ctx.createRadialGradient(cx, cy, 0, cx, cy, radius);
      gradient.addColorStop(0, `rgba(${rgb}, 0.038)`);
      gradient.addColorStop(0.55, `rgba(${rgb}, 0.014)`);
      gradient.addColorStop(1, `rgba(${rgb}, 0)`);
      ctx.fillStyle = gradient;
      ctx.beginPath();
      ctx.arc(cx, cy, radius, 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.restore();
  }

  drawPointer(now) {
    if (!this.ctx || !this.settings.drips || !this.pointer) return;
    const age = now - this.pointer.at;
    if (age > 2200) return;
    const fade = Math.max(0, 1 - age / 2200);
    const rgb = WATER_TONES[this.settings.tone] || WATER_TONES.ocean;
    const ctx = this.ctx;
    const gradient = ctx.createRadialGradient(
      this.pointer.x,
      this.pointer.y,
      0,
      this.pointer.x,
      this.pointer.y,
      74,
    );
    gradient.addColorStop(0, `rgba(${rgb}, ${0.055 * fade})`);
    gradient.addColorStop(0.45, `rgba(${rgb}, ${0.022 * fade})`);
    gradient.addColorStop(1, `rgba(${rgb}, 0)`);
    ctx.fillStyle = gradient;
    ctx.beginPath();
    ctx.arc(this.pointer.x, this.pointer.y, 74, 0, Math.PI * 2);
    ctx.fill();
  }

  frame = (now) => {
    if (!this.running || !this.ctx) return;
    const dt = Math.min(48, now - this.lastFrame);
    this.lastFrame = now;
    const ctx = this.ctx;
    const rgb = WATER_TONES[this.settings.tone] || WATER_TONES.ocean;
    const speed = WATER_SPEEDS[this.settings.speed] || 1;

    ctx.clearRect(0, 0, this.w, this.h);
    this.drawCaustics(now);
    this.drawPointer(now);

    const alive = [];
    for (const particle of this.particles) {
      particle.age += dt * speed;
      const progress = particle.age / particle.life;
      if (progress >= 1) continue;
      const eased = 1 - Math.pow(1 - progress, 3);
      const radius = particle.start + particle.grow * eased;
      const alpha = particle.alpha * Math.pow(1 - progress, 1.6);
      const squash = 1 + 0.055 * Math.sin(progress * Math.PI * 2 + particle.wobble);

      ctx.beginPath();
      ctx.ellipse(particle.x, particle.y, radius, radius * squash, 0, 0, Math.PI * 2);
      ctx.strokeStyle = `rgba(${rgb}, ${alpha})`;
      ctx.lineWidth = Math.max(0.55, particle.width * (1 - progress * 0.55));
      ctx.stroke();

      if (progress < 0.42) {
        const core = 1 - progress / 0.42;
        ctx.beginPath();
        ctx.ellipse(
          particle.x,
          particle.y,
          1.5 + radius * 0.06,
          1.5 + radius * 0.09,
          0,
          0,
          Math.PI * 2,
        );
        ctx.fillStyle = `rgba(${rgb}, ${alpha * 0.42 * core})`;
        ctx.fill();
      }
      alive.push(particle);
    }
    this.particles = alive;
    this.raf = requestAnimationFrame(this.frame);
  };
}

export default WaterSurface;
