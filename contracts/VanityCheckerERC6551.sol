// SPDX-License-Identifier: MIT
pragma solidity ^0.8.4;

contract VanityCheckerERC6551 {
    function getCreationCodeTBA(
        address implementation,
        bytes32 salt,
        uint256 chainId,
        address tokenContract,
        uint256 tokenId
    ) private pure returns (bytes memory result) {
        assembly {
            result := mload(0x40)
            mstore(add(result, 0xb7), tokenId)
            mstore(add(result, 0x97), shr(96, shl(96, tokenContract)))
            mstore(add(result, 0x77), chainId)
            mstore(add(result, 0x57), salt)
            mstore(add(result, 0x37), 0x5af43d82803e903d91602b57fd5bf3)
            mstore(add(result, 0x28), implementation)
            mstore(
                add(result, 0x14),
                0x3d60ad80600a3d3981f3363d3d373d3d3d363d73
            )
            mstore(result, 0xb7)
            mstore(0x40, add(result, 0xd7))
        }
    }

    function computeAddressCreate2(
        bytes32 salt,
        bytes32 bytecodeHash,
        address deployer
    ) private pure returns (address result) {
        assembly {
            result := mload(0x40)
            mstore8(result, 0xff)
            mstore(add(result, 0x35), bytecodeHash)
            mstore(add(result, 0x01), shl(96, deployer))
            mstore(add(result, 0x15), salt)
            result := keccak256(result, 0x55)
        }
    }

    function computeAddressTBA(
        address registry,
        address _implementation,
        bytes32 _salt,
        uint256 chainId,
        address tokenContract,
        uint256 tokenId
    ) public pure returns (address) {
        bytes32 bytecodeHash = keccak256(
            getCreationCodeTBA(
                _implementation,
                _salt,
                chainId,
                tokenContract,
                tokenId
            )
        );

        return computeAddressCreate2(_salt, bytecodeHash, registry);
    }
}
