import './style.css'
import { gsap } from 'gsap'
import { ScrollTrigger } from 'gsap/ScrollTrigger'

gsap.registerPlugin(ScrollTrigger)

// ── Nav scroll border ────────────────────────────────────────────────────────

window.addEventListener('scroll', () => {
  document.querySelector('.nav').classList.toggle('scrolled', window.scrollY > 20)
}, { passive: true })

// ── Copy button ───────────────────────────────────────────────────────────────

document.getElementById('copyBtn').addEventListener('click', () => {
  const text = document.getElementById('installCmd').textContent
  navigator.clipboard.writeText(text).then(() => {
    const btn = document.getElementById('copyBtn')
    const label = btn.querySelector('.copy-label')
    btn.classList.add('copied')
    label.textContent = 'Copied!'
    setTimeout(() => {
      btn.classList.remove('copied')
      label.textContent = 'Copy'
    }, 2000)
  })
})

// ── Hero entrance ─────────────────────────────────────────────────────────────

const heroTl = gsap.timeline({ defaults: { ease: 'power3.out' } })

heroTl
  .to('.nav', { opacity: 1, duration: 0.6 })
  .to('.hero-eyebrow', { opacity: 1, y: 0, duration: 0.7 }, '-=0.2')
  .fromTo('.hero-headline',
    { opacity: 0, y: 28 },
    { opacity: 1, y: 0, duration: 0.8 },
    '-=0.4')
  .fromTo('.hero-sub',
    { opacity: 0, y: 20 },
    { opacity: 1, y: 0, duration: 0.7 },
    '-=0.5')
  .fromTo('.hero-install',
    { opacity: 0, y: 16 },
    { opacity: 1, y: 0, duration: 0.6 },
    '-=0.4')
  .fromTo('.hero-hint',
    { opacity: 0 },
    { opacity: 1, duration: 0.5 },
    '-=0.2')
  // Illustration container fades in
  .to('.hero-illustration', { opacity: 1, duration: 0.4 }, '-=0.1')
  // Notes appear staggered
  .to('.note', { opacity: 1, duration: 0.35, stagger: 0.1, ease: 'power2.out' })
  // Connection dots appear
  .to('.dot', { opacity: 0.9, duration: 0.2, stagger: 0.06 }, '-=0.3')
  // Add a callback to start the repeating line-draw loop
  .call(startLineLoop)

// ── SVG line-draw loop ────────────────────────────────────────────────────────

function startLineLoop() {
  const conns = gsap.utils.toArray('.conn')

  function drawCycle() {
    // Reset all lines to their full dashoffset (invisible)
    conns.forEach(el => {
      const len = parseFloat(el.getAttribute('stroke-dasharray'))
      gsap.set(el, { strokeDashoffset: len, opacity: 0 })
    })

    const tl = gsap.timeline({ onComplete: () => gsap.delayedCall(1.8, drawCycle) })

    // Lines draw in one by one
    tl.to(conns, {
      opacity: 0.3,
      strokeDashoffset: 0,
      duration: 0.65,
      stagger: 0.12,
      ease: 'power2.inOut',
    })

    // Dots pulse when all lines are drawn
    tl.to('.dot', {
      scale: 1.8,
      opacity: 1,
      transformOrigin: 'center center',
      duration: 0.25,
      stagger: 0.07,
      ease: 'power2.out',
    }, '-=0.1')
    tl.to('.dot', {
      scale: 1,
      opacity: 0.9,
      duration: 0.25,
      stagger: 0.07,
      ease: 'power2.in',
    })

    // Fade all lines out before next cycle
    tl.to(conns, {
      opacity: 0,
      duration: 0.5,
      ease: 'power2.in',
      delay: 0.6,
    })
  }

  drawCycle()
}

// ── Feature cards (scroll-triggered stagger) ──────────────────────────────────

gsap.to('.feature-card', {
  opacity: 1,
  y: 0,
  duration: 0.6,
  stagger: 0.08,
  ease: 'power2.out',
  scrollTrigger: {
    trigger: '.features-grid',
    start: 'top 80%',
  },
})

// ── Terminal block ────────────────────────────────────────────────────────────

gsap.to('.terminal', {
  opacity: 1,
  y: 0,
  duration: 0.7,
  ease: 'power2.out',
  scrollTrigger: {
    trigger: '.terminal',
    start: 'top 82%',
  },
})

// ── Platforms ─────────────────────────────────────────────────────────────────

gsap.to('.platforms-grid', {
  opacity: 1,
  y: 0,
  duration: 0.65,
  ease: 'power2.out',
  scrollTrigger: {
    trigger: '.platforms-grid',
    start: 'top 82%',
  },
})

gsap.to('.platforms-note', {
  opacity: 1,
  duration: 0.5,
  delay: 0.2,
  ease: 'power2.out',
  scrollTrigger: {
    trigger: '.platforms-note',
    start: 'top 90%',
  },
})

// ── Footer ────────────────────────────────────────────────────────────────────

gsap.to('.footer', {
  opacity: 1,
  duration: 0.5,
  ease: 'power2.out',
  scrollTrigger: {
    trigger: '.footer',
    start: 'top 95%',
  },
})
