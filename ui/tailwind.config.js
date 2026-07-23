/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{html,js,svelte,ts}'],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        bg: { DEFAULT: 'var(--bg)', panel: 'var(--bg-panel)', hover: 'var(--bg-hover)' },
        text: { DEFAULT: 'var(--text)', dim: 'var(--text-dim)' },
        accent: { DEFAULT: 'var(--accent)', dim: 'var(--accent-dim)' },
        success: 'var(--success)',
        warn: 'var(--warn)',
        error: 'var(--error)',
        border: 'var(--border)',
      },
      fontFamily: {
        sans: ['-apple-system', 'BlinkMacSystemFont', 'SF Pro Text', 'Inter', 'system-ui', 'sans-serif'],
        mono: ['SF Mono', 'JetBrains Mono', 'Menlo', 'monospace'],
      },
      borderRadius: {
        mac: '10px',
        'mac-sm': '6px',
        'mac-lg': '14px',
      },
      backdropBlur: {
        mac: '20px',
      },
      animation: {
        'fade-in': 'fadeIn 0.2s ease-out',
        'slide-up': 'slideUp 0.3s cubic-bezier(0.16, 1, 0.3, 1)',
        'scale-in': 'scaleIn 0.15s ease-out',
        'shimmer': 'shimmer 1.6s ease-in-out infinite',
        'bg-flow': 'bgFlow 20s ease infinite',
        'stagger-in': 'staggerIn 0.4s cubic-bezier(0.16, 1, 0.3, 1) forwards',
        'mode-slide': 'modeSlide 0.32s cubic-bezier(0.16, 1, 0.3, 1)',
        'pulse-soft': 'pulseSoft 2.4s cubic-bezier(0.4, 0, 0.6, 1) infinite',
      },
      keyframes: {
        fadeIn: { '0%': { opacity: '0' }, '100%': { opacity: '1' } },
        slideUp: { '0%': { opacity: '0', transform: 'translateY(8px)' }, '100%': { opacity: '1', transform: 'translateY(0)' } },
        scaleIn: { '0%': { opacity: '0', transform: 'scale(0.96)' }, '100%': { opacity: '1', transform: 'scale(1)' } },
        shimmer: { '0%': { transform: 'translateX(-100%)' }, '100%': { transform: 'translateX(100%)' } },
        bgFlow: { '0%': { 'background-position': '0% 50%' }, '50%': { 'background-position': '100% 50%' }, '100%': { 'background-position': '0% 50%' } },
        staggerIn: { '0%': { opacity: '0', transform: 'translateY(10px) scale(0.98)' }, '100%': { opacity: '1', transform: 'translateY(0) scale(1)' } },
        modeSlide: { '0%': { opacity: '0', transform: 'translateX(12px)' }, '100%': { opacity: '1', transform: 'translateX(0)' } },
        pulseSoft: { '0%, 100%': { opacity: '1' }, '50%': { opacity: '0.6' } },
      },
    },
  },
  plugins: [],
}
