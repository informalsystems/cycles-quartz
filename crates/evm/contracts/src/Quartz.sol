// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.13;

import "@automata-dcap/interfaces/IAttestation.sol";

// QUESTION - We do not test that mr_enclave matches yet in cosmwasm. Should we do it here?
// QUESTION - I guess for the transfers app, or ping pong, they would actually inherit the Quartz contract? Right?
// this way whenever those functions are called on ping pong, they would increase the sequence number (I assume
// the handshake does not update the sequence number)
// QUESTION - Contract address MIGHT be put into the config? (but I dont think that matters on the solidity side as its available globally in the contract)

contract Quartz {
    Config public config;
    bytes32 public enclavePubKey;
    uint256 public sequenceNum;
    IAttestation attest = IAttestation(0x76A3657F2d6c5C66733e9b69ACaDadCd0B68788b); // Sepolia address - can set in constructor for other networks
    // NOTE - nonce is no longer needed

    struct Config {
        bytes32 mrEnclave;
        LightClientOpts lightClientOpts;
        address pccs; // is both the tcbinfo_contract and dcap_verifier_contract of cosmwasm
    }

    // TODO Shoaib to figure out real values
    struct LightClientOpts {
        string chainID;
        uint256 trustedHeight;
        bytes32 trustedHash;
    }
    // etc.

    event SessionCreated(address indexed quartz);
    event PubKeySet(bytes32 indexed enclavePubKey);

    /**
     * @dev Modifier that verifies the caller's authenticity through an enclave-attested quote.
     * Reverts with a specific error message if attestation fails.
     * @param _quote The attestation quote used to verify the caller's enclave status.
     */
    modifier onlyEnclave(bytes memory _quote) {
        (bool success, bytes memory output) = attest.verifyAndAttestOnChain(_quote);
        if (success) {
            _;
        } else {
            string memory errorMessage = _getRevertMessage(output);
            revert(errorMessage);
        }
    }

    /**
     * @notice Initializes the Quartz contract with the config, attests it's from a DCAP enclave,
     * and emits an event for the host to listen to
     * @dev On failure the contract will not deploy, and the user will lose the gas. The constructor
     * is equivalent to the start of the handshake, and the session create, as the event emitted
     * can be passed onto the host to share with the enclave
     * @param _config The configuration object for the light client
     * @param _quote The DCAP attestation quote provided by the enclave
     * Emits a {SessionCreated} event upon successful verification.
     * Reverts as per onlyEnclave()
     */
    constructor(Config memory _config, bytes memory _quote) onlyEnclave(_quote) {
        config = _config;
        emit SessionCreated(address(this));
    }

    /**
     * @notice Sets the session public key after verifying the attestation quote
     * @dev This function is equivalent to cosmwasm setting of the session, without the nonce, since
     * the nonce is no longer needed
     * @param _pubKey The public key to be set for the session, provided by the enclave
     * @param _quote The attestation quote to be verified, ensuring that the caller is authorized
     * Emits a {PubKeySet} event upon successful setting of the public key
     * Reverts with an error message if `verifyAndAttestOnChain` fails to verify the attestation
     */
    function setSessionPubKey(bytes32 _pubKey, bytes memory _quote) external onlyEnclave(_quote) {
        enclavePubKey = _pubKey;
        emit PubKeySet(enclavePubKey);
    }

    // TODO - Implement sequence number incrementing... but I assume we should have the transfers or ping pong app do this

    /**
     * @notice Extracts the revert message from a failed external call's return data
     * @param _output The raw return data from a failed external call
     * @return The string representing the revert message
     */
    function _getRevertMessage(bytes memory _output) internal pure returns (string memory) {
        if (_output.length == 0) {
            return "Unknown error";
        }
        assembly {
            // Skip the first 4 bytes (error selector)
            _output := add(_output, 0x04)
        }
        return abi.decode(_output, (string));
    }
}
