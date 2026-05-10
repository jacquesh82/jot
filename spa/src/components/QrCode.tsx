import { useEffect, useRef } from "preact/hooks";
import QRCode from "qrcode";

interface Props { text: string; size?: number }

export function QrCode({ text, size = 180 }: Props) {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    if (ref.current && text) {
      QRCode.toCanvas(ref.current, text, {
        width: size,
        margin: 2,
        color: { dark: "#000000", light: "#ffffff" },
      });
    }
  }, [text, size]);

  return <canvas ref={ref} class="qr-canvas" width={size} height={size} />;
}
