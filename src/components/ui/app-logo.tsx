import tokenSvg from "@argus/public/token.svg?raw";

import { cn } from "../../lib/cn";

type AppLogoProps = {
  size?: number;
  className?: string;
  style?: React.CSSProperties;
};

export function AppLogo({ size = 22, className, style }: AppLogoProps) {
  const markup = tokenSvg.replace(
    "<svg",
    `<svg width="${size}" height="${size}"`,
  );

  return (
    <span
      className={cn("inline-flex shrink-0 text-current", className)}
      style={{ width: size, height: size, ...style }}
      aria-hidden
      dangerouslySetInnerHTML={{ __html: markup }}
    />
  );
}
