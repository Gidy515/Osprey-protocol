import Image from "next/image";

type OspreyLogoProps = {
  size?: number;
  showWordmark?: boolean;
};

export function OspreyLogo({
  size = 38,
  showWordmark = true,
}: OspreyLogoProps) {
  return (
    <div className="osprey-logo" aria-label="Osprey Protocol">
      <Image
        src="/brand/osprey-logo.png"
        alt=""
        width={size}
        height={size}
        aria-hidden="true"
        priority
        style={{
          width: size,
          height: size,
          objectFit: "contain",
          borderRadius: "50%",
        }}
      />

      {showWordmark && <strong>OSPREY</strong>}
    </div>
  );
}
