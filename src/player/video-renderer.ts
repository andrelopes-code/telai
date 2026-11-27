/**
 * Draws JPEG frames on a canvas. Frames that arrive while one is decoding
 * replace each other, so only the newest is drawn next.
 */
export class VideoRenderer {
  private readonly context: CanvasRenderingContext2D;
  private pending: Uint8Array | null = null;
  private decoding = false;

  constructor(private readonly canvas: HTMLCanvasElement) {
    const context = canvas.getContext("2d", { alpha: false });
    if (!context) {
      throw new Error("2D canvas is unavailable");
    }

    this.context = context;
  }

  draw(jpeg: Uint8Array): void {
    this.pending = jpeg;

    if (!this.decoding) {
      void this.drawPending();
    }
  }

  clear(): void {
    this.pending = null;
    this.context.fillStyle = "#000";
    this.context.fillRect(0, 0, this.canvas.width, this.canvas.height);
  }

  private async drawPending(): Promise<void> {
    this.decoding = true;

    while (this.pending) {
      const jpeg = this.pending;
      this.pending = null;

      try {
        const bitmap = await createImageBitmap(new Blob([jpeg as Uint8Array<ArrayBuffer>]));
        this.resize(bitmap.width, bitmap.height);
        this.context.drawImage(bitmap, 0, 0);
        bitmap.close();
      } catch {
        // A frame that fails to decode is skipped; the next one replaces it.
      }
    }

    this.decoding = false;
  }

  private resize(width: number, height: number): void {
    if (this.canvas.width !== width || this.canvas.height !== height) {
      this.canvas.width = width;
      this.canvas.height = height;
    }
  }
}
