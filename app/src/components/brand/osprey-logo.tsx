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
      <svg
        width={size}
        height={size}
        viewBox="0 0 64 64"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
        aria-hidden="true"
      >
        <circle
          cx="32"
          cy="32"
          r="28"
          stroke="#C86F3D"
          strokeWidth="3"
        />

        <path
          d="M18.5 34.5C21.5 25.2 29 18.5 40.7 18.8C37.5 21.1 35.4 23.6 34.1 26.4C38.4 25.5 42.4 26.3 46.5 29C42.1 29.3 39.2 30.6 37.3 32.9C35.2 35.5 34.6 39 34.7 43.4C31.8 40.1 29.4 37.6 26.7 36.2C24.4 35 21.8 34.5 18.5 34.5Z"
          fill="#F4F0E8"
        />

        <path
          d="M34.3 27.2C37.7 26.6 40.7 27.1 43.8 29C40.6 29.5 38.3 30.6 36.8 32.5C35.2 34.4 34.3 37 34.1 40.1C32.4 37.6 30.8 35.6 28.9 34.3C31.3 32.6 33 30.2 34.3 27.2Z"
          fill="#C86F3D"
        />

        <path
          d="M43.8 29L50.2 31.1L43.4 32.6C44.1 31.5 44.2 30.3 43.8 29Z"
          fill="#F4F0E8"
        />

        <circle cx="38.2" cy="27.7" r="1.25" fill="#111210" />

        <path
          d="M21 35.2C25.4 36.2 28.9 38.4 32.8 43.8"
          stroke="#C86F3D"
          strokeWidth="2"
          strokeLinecap="round"
        />
      </svg>

      {showWordmark && <strong>OSPREY</strong>}
    </div>
  );
}
