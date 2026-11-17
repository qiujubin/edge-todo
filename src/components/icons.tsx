/** 极简线性图标（1.5px 描边，HUD 风） */
import type { JSX } from "solid-js";

function Svg(props: { children: JSX.Element; viewBox?: string }) {
  return (
    <svg
      viewBox={props.viewBox ?? "0 0 16 16"}
      fill="none"
      stroke="currentColor"
      stroke-width="1.5"
      stroke-linecap="round"
      stroke-linejoin="round"
    >
      {props.children}
    </svg>
  );
}

export const IconPin = () => (
  <Svg>
    <path d="M9.5 1.5 14.5 6.5 12 9 12 12 4 4 7 4z" transform="rotate(45 9 7)" />
    <path d="M4 12 L2 14" />
  </Svg>
);

export const IconCollapse = () => (
  <Svg>
    <path d="M6 3 L11 8 L6 13" />
  </Svg>
);

export const IconPlus = () => (
  <Svg>
    <path d="M8 3 V13 M3 8 H13" />
  </Svg>
);

export const IconClock = () => (
  <Svg>
    <circle cx="8" cy="8" r="5.5" />
    <path d="M8 5 V8 L10 9.5" />
  </Svg>
);

export const IconRepeat = () => (
  <Svg>
    <path d="M3 6 A5 5 0 0 1 13 6 M13 3 V6 H10 M13 10 A5 5 0 0 1 3 10 M3 13 V10 H6" />
  </Svg>
);

export const IconPostpone = () => (
  <Svg>
    <path d="M13 8 A5 5 0 1 1 8 3 M8 3 H11 M8 3 V6" transform="" />
    <path d="M10.5 1.5 L13 3 L11 5" />
  </Svg>
);

export const IconEdit = () => (
  <Svg>
    <path d="M11.5 1.5 14.5 4.5 5.5 13.5 2 14.5 3 11z" />
  </Svg>
);

export const IconTrash = () => (
  <Svg>
    <path d="M3 4.5 H13 M6.5 4.5 V3 H9.5 V4.5 M4.5 4.5 L5.2 13.5 H10.8 L11.5 4.5 M6.8 7 V11 M9.2 7 V11" />
  </Svg>
);

export const IconChevron = () => (
  <Svg>
    <path d="M4 6 L8 10 L12 6" />
  </Svg>
);

export const IconTomorrow = () => (
  <Svg>
    <rect x="2.5" y="3.5" width="11" height="10" rx="1.5" />
    <path d="M2.5 7 H13.5 M5.5 2 V5 M10.5 2 V5 M8 9 V11 M7 10 H9" />
  </Svg>
);
