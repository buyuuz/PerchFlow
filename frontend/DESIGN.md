---
name: Perch Design System
colors:
  surface: '#0e1416'
  surface-dim: '#0e1416'
  surface-bright: '#343a3c'
  surface-container-lowest: '#090f11'
  surface-container-low: '#161d1e'
  surface-container: '#1a2122'
  surface-container-high: '#252b2c'
  surface-container-highest: '#2f3637'
  on-surface: '#dde3e5'
  on-surface-variant: '#bbc9cc'
  inverse-surface: '#dde3e5'
  inverse-on-surface: '#2b3133'
  outline: '#869396'
  outline-variant: '#3c494c'
  surface-tint: '#44d8f1'
  primary: '#44d8f1'
  on-primary: '#00363e'
  primary-container: '#00bcd4'
  on-primary-container: '#004650'
  inverse-primary: '#006876'
  secondary: '#adcbda'
  on-secondary: '#163440'
  secondary-container: '#304d5a'
  on-secondary-container: '#9fbdcc'
  tertiary: '#ffb87b'
  on-tertiary: '#4c2700'
  tertiary-container: '#f19640'
  on-tertiary-container: '#633400'
  error: '#ffb4ab'
  on-error: '#690005'
  error-container: '#93000a'
  on-error-container: '#ffdad6'
  primary-fixed: '#a1efff'
  primary-fixed-dim: '#44d8f1'
  on-primary-fixed: '#001f25'
  on-primary-fixed-variant: '#004e59'
  secondary-fixed: '#c9e7f7'
  secondary-fixed-dim: '#adcbda'
  on-secondary-fixed: '#001f2a'
  on-secondary-fixed-variant: '#2e4b57'
  tertiary-fixed: '#ffdcc2'
  tertiary-fixed-dim: '#ffb77b'
  on-tertiary-fixed: '#2e1500'
  on-tertiary-fixed-variant: '#6d3a00'
  background: '#0e1416'
  on-background: '#dde3e5'
  surface-variant: '#2f3637'
typography:
  headline-lg:
    fontFamily: Inter
    fontSize: 24px
    fontWeight: '600'
    lineHeight: 32px
    letterSpacing: -0.02em
  headline-md:
    fontFamily: Inter
    fontSize: 18px
    fontWeight: '600'
    lineHeight: 24px
  body-md:
    fontFamily: Inter
    fontSize: 14px
    fontWeight: '400'
    lineHeight: 20px
  body-sm:
    fontFamily: Inter
    fontSize: 12px
    fontWeight: '400'
    lineHeight: 16px
  mono-label:
    fontFamily: JetBrains Mono
    fontSize: 13px
    fontWeight: '500'
    lineHeight: 16px
  mono-data:
    fontFamily: JetBrains Mono
    fontSize: 12px
    fontWeight: '400'
    lineHeight: 18px
rounded:
  sm: 0.25rem
  DEFAULT: 0.5rem
  md: 0.75rem
  lg: 1rem
  xl: 1.5rem
  full: 9999px
spacing:
  unit: 4px
  container-padding: 24px
  gutter: 16px
  card-gap: 12px
---

## Brand & Style

This design system is engineered for **Perch**, a technical utility for Windows process and service orchestration. The brand personality is precise, transparent, and sophisticated, favoring a "tool-first" mentality that prioritizes system data over marketing aesthetics.

The visual style employs **Glassmorphism** with a refined, Apple-inspired execution. It focuses on depth through translucency, background blurs, and light-refracting strokes. The goal is to make a dense technical interface feel lightweight and integrated into the OS environment. The emotional response should be one of "calm control"—providing a high-density information environment that remains legible and professional.

Key principles:
- **Optical Clarity:** High-quality background blurs (30px+) ensure text legibility over diverse backgrounds.
- **Precision:** Fine 1px borders mimic the physical properties of glass edges.
- **Technical Sincerity:** Use of monospaced fonts for all system-generated data to ensure character alignment and a "developer-centric" feel.

## Colors

The palette is anchored by a **Calm Cyan** primary accent, chosen for its high visibility against dark glass surfaces without causing eye fatigue. 

- **Background:** A deep, low-saturation cool gray-blue gradient (`#0F172A` to `#1E293B`) provides the "desktop" layer.
- **Accent:** Cyan (#00BCD4) is used strictly for primary actions, focus states, and active process indicators.
- **Surfaces:** Cards use semi-transparent whites (`rgba(255, 255, 255, 0.06)`) in dark mode, paired with a `backdrop-filter: blur(30px)`.
- **Feedback:** 
    - **Green (#4CAF50):** Operational, Connected, Success.
    - **Amber (#FFB300):** Transitioning, Restarting, Warning.
    - **Red (#F44336):** Terminated, Error, Critical Failure.

## Typography

The system utilizes a dual-font strategy to balance UI navigation with technical data density.

- **UI Language:** **Inter** is used for all functional labels, headers, and navigation elements. It provides excellent legibility for Simplified Chinese characters at small scales.
- **Technical Data:** **JetBrains Mono** is used for all system values, including PIDs, CLI arguments, memory usage, and logs. This ensures that columns of numbers align perfectly and look distinct from the UI controls.
- **Language Optimization:** For Simplified Chinese, the line-height is slightly increased (min 1.5x) to ensure complex characters remain clear within the frosted glass containers.

## Layout & Spacing

This design system uses a **Fluid Grid** model with a 4px base unit. 

- **Information Density:** Content is packed tightly to allow power users to monitor dozens of services simultaneously, but horizontal breathing room is maintained through generous 24px side margins.
- **Grid:** A 12-column system is used for the main dashboard. Sidebar navigation is fixed at 240px, while the main content area expands.
- **Desktop/Tablet:** On larger screens, process cards can be viewed in a multi-column grid. On narrower widths, the layout reflows into a single-column list view with vertical density.

## Elevation & Depth

Depth is conveyed through **Backdrop Refraction** rather than traditional drop shadows.

- **Layer 0 (Desktop):** The base gradient background.
- **Layer 1 (Cards):** 30px backdrop blur, 6% white opacity fill, and a 1px inner stroke (`rgba(255, 255, 255, 0.1)`) to define the edge.
- **Layer 2 (Popovers/Modals):** 40px backdrop blur, 12% white opacity fill, and a soft ambient shadow (`0 8px 32px rgba(0,0,0,0.3)`) to separate the element from the card layer.
- **Interactions:** Upon hover, card opacity increases from 6% to 10% to create a "glow" effect without moving the element.

## Shapes

The shape language is **Rounded (0.5rem)**, echoing the modern Windows 11 and macOS aesthetics. 

- **Standard Containers:** 8px (0.5rem) corner radius.
- **Large Cards/Modals:** 16px (1rem) corner radius.
- **Technical Badges:** 4px (0.25rem) to maintain a crisp, functional appearance.
- **Search Inputs:** Fully rounded (pill-shaped) to distinguish them from data containers.

## Components

- **Frosted Glass Cards:** Use as the primary container for process details. Should include a 1px "highlight" border on the top and left edges to simulate light hitting glass.
- **Technical Badges:** Small, rectangular badges with a subtle background tint (e.g., 10% Green for "Running"). Use JetBrains Mono for the text.
- **Log Viewer:** A dedicated terminal component with a `#000000` 40% opacity background. Text should be `mono-data` in a light gray or off-white. Use Cyan for the cursor or active line.
- **Glass Toggles:** Switches that use a frosted background track and a solid Cyan thumb when active.
- **Process Icons:** The Perch logo should be a minimalist, linear bird icon, rendered in 1.5px stroke weight Cyan.
- **Action Buttons:** Ghost buttons with Cyan borders for secondary actions; solid Cyan buttons with white text for primary actions like "Start Service."
- **Data Tables:** Row-based layouts with subtle 1px dividers (`rgba(255, 255, 255, 0.05)`). No heavy vertical borders.