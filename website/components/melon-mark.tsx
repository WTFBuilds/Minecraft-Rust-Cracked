export function MelonMark({ className = "h-7 w-7" }: { className?: string }) {
  return (
    <svg viewBox="0 0 32 32" className={className} aria-hidden="true">
      <rect width="32" height="32" rx="7" fill="#0f1a0f" />
      <rect x="2" y="8" width="28" height="4" fill="#3a8c1c" />
      <rect x="4" y="12" width="24" height="4" fill="#56b826" />
      <rect x="6" y="16" width="20" height="4" fill="#ff4f70" />
      <rect x="8" y="20" width="16" height="4" fill="#ff4f70" />
      <rect x="10" y="24" width="12" height="2" fill="#ff7a92" />
      <rect x="11" y="16" width="2" height="2" fill="#0a120a" />
      <rect x="16" y="18" width="2" height="2" fill="#0a120a" />
      <rect x="21" y="16" width="2" height="2" fill="#0a120a" />
      <rect x="13" y="21" width="2" height="2" fill="#0a120a" />
      <rect x="19" y="21" width="2" height="2" fill="#0a120a" />
    </svg>
  );
}
