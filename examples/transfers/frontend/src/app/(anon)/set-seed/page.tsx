'use client'

import { useState } from 'react'
import { useRouter } from 'next/navigation'

import { EnterSeedModal } from '@/components/EnterSeedModal'
import { generateMnemonic, saveMnemonic } from '@/lib/ephemeralKeypair'
import { StyledText } from '@/components/StyledText'
import { useGlobalState } from '@/state/useGlobalState'

const mnemonic = generateMnemonic()

export default function SetSeed() {
  const router = useRouter()
  const [isModalOpen, setIsModalOpen] = useState(false)

  const acceptPhrase = () => {
    useGlobalState.getState().setLoading(true)
    saveMnemonic(mnemonic)
    router.replace('/dashboard')
  }

  return (
    <main className="flex min-h-screen flex-col items-center gap-8 p-24">
      <h1>
        This will be your recovery seed phrase for your public/private keys:
      </h1>
      <code className="rounded bg-slate-500 p-2 font-bold text-white">
        {mnemonic}
      </code>
      <div className="flex flex-col gap-4">
        <StyledText
          as="button"
          variant="button.primary"
          onClick={acceptPhrase}
        >
          Continue with the autogenerated seed phrase
        </StyledText>
        <StyledText
          as="button"
          variant="button.secondary"
          onClick={() => setIsModalOpen(true)}
        >
          I want to enter my own recovery phrase instead
        </StyledText>
      </div>
      <EnterSeedModal
        isOpen={isModalOpen}
        onClose={() => setIsModalOpen(false)}
      />
    </main>
  )
}
