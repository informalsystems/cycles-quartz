# TEEs

## Why TEEs?

TEEs allow you to do things that cryptography alone doesnt - like verifiable
data deletion, decentralized front-end hosting, collusion resistance, etc.

They also let you overcome current deficiencies and inefficiencies in
cryptographic solutions. TEEs are best seen as complements to a cryptographic stack.

A major motivation for TEEs is private computation. There are really only two ways to do computation on private data: MPC and TEEs.

- MPC is still highly inefficient for complex transactions. 
- FHE is just a way to accelerate MPC (trading off network IO for compute). It's not a privacy solution for blockchains on its own.
- ZKP provide privacy from the verifier but not from the prover. Producing a ZKP privately either requires MPC or a TEE.
- Ideal stack combines all privacy technologies as appropriate.

The goal with Quartz is to provide a simple framework for getting started using
TEEs with an eye towards reducing dependency on the TEE as much as possible
(using light client protocols, ZKPs, etc.)

## Resources on TEE security.

For a great technical background on SGX, see [Intel SGX
Explained](https://eprint.iacr.org/2016/086.pdf).

This paper contains an infamous quote:

> our security analysis reveals that the limitations in SGX’s guarantees mean that a security conscious software developer cannot in good conscience
rely on SGX for secure remote computation

The core concern was about how SGX remote attestation works, using an old system
called EPID which had a high dependence on Intel for liveness. EPID has since been deprecated and the crux of the concerns have
been addressed by the new remote attestation scheme, called DCAP.

For more essential getting started resources, see Andrew Miller's [Getting
Started in SGX](https://flashbots.notion.site/Getting-started-in-SGX-2ec697048e5d458fb0230e75f9d064c7).

See also the following talks:

- Andrew Miller - [The TEE Stack][tee-stack]
- Sylvain Bellemare - [Moving Towards Open Source & Verifiable Secure-through-Physics TEE Chips][bellemare-tee-salon] 
- Ethan Buchman - [How to Win Friends and TEE-fluence People][how-to-win-friends]


[how-to-win-friends]: https://www.youtube.com/watch?v=XwKIt5XYyqw
[tee-stack]: https://www.youtube.com/watch?v=9AwlMB8TF4o
[bellemare-tee-salon]: https://www.youtube.com/watch?v=j6pGxMfffdA
