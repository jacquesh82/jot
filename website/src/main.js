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
