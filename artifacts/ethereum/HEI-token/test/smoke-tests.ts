import { expect } from "chai";
import { ethers } from "hardhat";
import { loadFixture } from "@nomicfoundation/hardhat-network-helpers";
import { keccak256 } from "ethers";

describe("HEI", function () {
  const MINT_ROLE = keccak256(
    new ethers.AbiCoder().encode(["string"], ["MINTER"])
  );

  const BLACK_HOLE_ADDRESS = "0x000000000000000000000000000000000000dEaD";
  async function deployTerFixture() {
    const [admin, minter, user, user2] = await ethers.getSigners();

    const UnderlyingToken = await ethers.deployContract("ERC20Test", [
      "UnderlyingToken",
      "LIT",
      ethers.parseUnits("100000000", 18), // 100 million
      admin.address,
    ]);
    await expect(
      ethers.deployContract("HEI", [
        ethers.ZeroAddress,
        "heima",
        "HEI",
        admin.address,
      ])
    ).to.be.revertedWith("Invalid underlyingToken address");
    const HEI = await ethers.deployContract("HEI", [
      UnderlyingToken.target,
      "heima",
      "HEI",
      admin.address,
    ]);

    console.log("admin address", admin.address);
    console.log("minter address", minter.address);
    console.log("user address", user.address);
    console.log("user2 address", user2.address);
    console.log("HEI address", HEI.target);
    console.log("UnderlyingToken address", UnderlyingToken.target);

    return { admin, minter, user, user2, HEI, UnderlyingToken };
  }

  it("Contracts should deploy successfully", async function () {
    const { HEI, UnderlyingToken } = await loadFixture(deployTerFixture);
    expect(UnderlyingToken, "UnderlyingToken contract should be deployed").to.be
      .ok;
    expect(HEI, "HEI contract should be deployed").to.be.ok;
  });

  it("Checking initial state", async function () {
    const { admin, HEI, UnderlyingToken } = await loadFixture(deployTerFixture);
    expect(
      await UnderlyingToken.balanceOf(HEI.target),
      "HEI should have 0 balance"
    ).to.equal(0);
    expect(
      await UnderlyingToken.balanceOf(admin.address),
      "Admin should have 0 balance of UnderlyingToken"
    ).to.equal(0);
    expect(
      await UnderlyingToken.totalSupply(),
      "UnderlyingToken should have 0 total supply"
    ).to.equal(0);
    expect(
      await HEI.balanceOf(admin.address),
      "Admin should have 0 balance of HEI"
    ).to.equal(0);
    expect(await HEI.totalSupply(), "HEI should have 0 total supply").to.equal(
      0
    );
    expect(await HEI.underlying(), "HEI underlying should be correct").to.equal(
      UnderlyingToken.target
    );
    expect(await HEI.MINT_ROLE(), "MINT_ROLE should be correct").to.equal(
      MINT_ROLE
    );
    expect(
      await HEI.DEFAULT_ADMIN_ROLE(),
      "DEFAULT_ADMIN_ROLE should be correct"
    ).to.equal(
      "0x0000000000000000000000000000000000000000000000000000000000000000"
    );
    expect(
      await HEI.BLACK_HOLE_ADDRESS(),
      "BLACK_HOLE_ADDRESS should be correct"
    ).to.equal(BLACK_HOLE_ADDRESS);
  });

  it("Grant a minter", async function () {
    const { admin, minter, HEI, user } = await loadFixture(deployTerFixture);

    expect(
      await HEI.hasRole(await HEI.MINT_ROLE(), minter.address),
      "Minter should have MINT_ROLE after being granted"
    ).to.be.false;

    await HEI.connect(admin).grantMinter(minter.address);

    expect(
      await HEI.hasRole(await HEI.MINT_ROLE(), minter.address),
      "Minter should have MINT_ROLE after being granted"
    ).to.be.true;
    expect(
      await HEI.hasRole(await HEI.MINT_ROLE(), user.address),
      "User should not have the mint role"
    ).to.be.false;

    // Mint tokens
    expect(
      await HEI.balanceOf(user.address),
      "User should have 0 balance before minting"
    ).to.equal(0);

    await HEI.connect(minter).mint(user.address, ethers.parseUnits("100", 18));
    // Verify balance
    expect(
      await HEI.balanceOf(user.address),
      "User should have correct balance after minting"
    ).to.equal(ethers.parseUnits("100", 18));
    expect(
      await HEI.totalSupply(),
      "HEI total supply should be correct"
    ).to.equal(ethers.parseUnits("100", 18));
  });

  it("Should not allow non-admin to grant minter", async function () {
    const { minter, HEI, user } = await loadFixture(deployTerFixture);
    await expect(HEI.connect(user).grantMinter(minter.address))
      .to.be.revertedWithCustomError(HEI, "AccessControlUnauthorizedAccount")
      .withArgs(
        user.address,
        "0x0000000000000000000000000000000000000000000000000000000000000000"
      );
  });

  it("Should allow default admin to change", async function () {
    const { admin, minter, HEI, user } = await loadFixture(deployTerFixture);
    expect(
      await HEI.defaultAdmin(),
      "Default admin should be correct"
    ).to.equal(admin.address);
    await HEI.connect(admin).beginDefaultAdminTransfer(minter.address);
    await HEI.connect(minter).acceptDefaultAdminTransfer();
    expect(
      await HEI.defaultAdmin(),
      "Default admin should be correct"
    ).to.equal(minter.address);
  });

  it("Should not allow non-admin to change default admin", async function () {
    const { minter, HEI, user } = await loadFixture(deployTerFixture);
    await expect(HEI.connect(user).beginDefaultAdminTransfer(minter.address))
      .to.be.revertedWithCustomError(HEI, "AccessControlUnauthorizedAccount")
      .withArgs(
        user.address,
        "0x0000000000000000000000000000000000000000000000000000000000000000"
      );
  });

  it("Should allow rollback of admin transfer", async function () {
    const { admin, minter, HEI } = await loadFixture(deployTerFixture);
    expect(await HEI.defaultAdmin()).to.equal(admin.address);
    await HEI.connect(admin).beginDefaultAdminTransfer(minter.address);

    await HEI.connect(admin).rollbackDefaultAdminDelay();

    expect(await HEI.defaultAdmin()).to.equal(admin.address);
  });

  it("Should not allow non-admin to rollback", async function () {
    const { admin, minter, HEI, user } = await loadFixture(deployTerFixture);

    await HEI.connect(admin).beginDefaultAdminTransfer(minter.address);

    await expect(
      HEI.connect(user).rollbackDefaultAdminDelay()
    ).to.be.revertedWithCustomError(HEI, "AccessControlUnauthorizedAccount");
  });

  it("Should revert if the minter is invalid", async function () {
    const { admin, minter, HEI, user } = await loadFixture(deployTerFixture);
    await HEI.connect(admin).grantMinter(minter.address);
    await expect(
      HEI.connect(user).mint(ethers.ZeroAddress, ethers.parseUnits("100", 18))
    )
      .to.be.revertedWithCustomError(HEI, "AccessControlUnauthorizedAccount")
      .withArgs(user.address, MINT_ROLE);

    expect(
      await HEI.balanceOf(ethers.ZeroAddress),
      "ZeroAddress should have 0 balance of HEI"
    ).to.equal(0);
    await expect(HEI.connect(admin).grantMinter(ethers.ZeroAddress))
      .to.be.revertedWithCustomError(HEI, "AccessControlInvalidMinter")
      .withArgs(ethers.ZeroAddress);
  });

  it("Deposit underlying tokens", async function () {
    const { admin, user, HEI, UnderlyingToken } = await loadFixture(
      deployTerFixture
    );
    const depositAmount = ethers.parseUnits("100", 18);

    // Mint underlying tokens to user
    await UnderlyingToken.connect(admin).mint(
      user.address,
      ethers.parseUnits("1000", 18)
    );
    expect(await UnderlyingToken.balanceOf(user.address)).to.equal(
      ethers.parseUnits("1000", 18),
      "User should have 1000 underlying tokens after minting"
    );
    // Approve the HEI contract to spend user's underlying tokens
    await UnderlyingToken.connect(user).approve(
      HEI.target,
      ethers.parseUnits("1000", 18)
    );

    // Check initial balances
    expect(await HEI.balanceOf(user.address)).to.equal(0);

    // Perform the deposit
    await HEI.connect(user).depositFor(user.address, depositAmount);

    // Check final balances
    expect(await UnderlyingToken.balanceOf(user.address)).to.equal(
      ethers.parseUnits("900", 18),
      "User should have 900 underlying tokens after deposit"
    );
    expect(await HEI.balanceOf(user.address)).to.equal(
      depositAmount,
      "User should have correct balance of HEI after deposit"
    );
    expect(await HEI.totalSupply()).to.equal(
      depositAmount,
      "HEI total supply should be correct"
    );

    expect(
      await UnderlyingToken.balanceOf(BLACK_HOLE_ADDRESS),
      "HEI should have correct balance of underlying tokens"
    ).to.equal(depositAmount);
  });

  it("Burn tokens", async function () {
    const { admin, minter, HEI, user } = await loadFixture(deployTerFixture);

    // 1. Grant minter role to Minter
    await HEI.connect(admin).grantMinter(minter.address);
    // 2. Mint tokens to user
    await HEI.connect(minter).mint(user.address, ethers.parseUnits("100", 18));
    // 3. Check user's balance
    expect(await HEI.balanceOf(user.address)).to.equal(
      ethers.parseUnits("100", 18),
      "User should have 100 HEI tokens"
    );
    // 4. Burn tokens from user's account
    await HEI.connect(user).burn(ethers.parseUnits("100", 18));
    // 5. Verify final balance
    expect(await HEI.balanceOf(user.address)).to.equal(
      0,
      "User should have 0 HEI tokens after burning"
    );
  });

  it("Should allow burning tokens from another account with approval", async function () {
    const { admin, minter, HEI, user } = await loadFixture(deployTerFixture);
    // 1. First mint some tokens to the user
    await HEI.connect(admin).grantMinter(minter.address);
    await HEI.connect(minter).mint(user.address, ethers.parseUnits("100", 18));

    // 2. User approves minter to burn tokens
    await HEI.connect(user).approve(
      minter.address,
      ethers.parseUnits("50", 18)
    );

    // 3. Check initial balance
    expect(await HEI.balanceOf(user.address)).to.equal(
      ethers.parseUnits("100", 18)
    );

    // 4. Minter burns tokens from user's account
    await HEI.connect(minter).burnFrom(
      user.address,
      ethers.parseUnits("50", 18)
    );

    // 5. Verify final balance
    expect(await HEI.balanceOf(user.address)).to.equal(
      ethers.parseUnits("50", 18)
    );
  });
  it("Should not allow burning more than approvals", async function () {
    const { minter, HEI, user } = await loadFixture(deployTerFixture);
    await HEI.connect(user).approve(
      minter.address,
      ethers.parseUnits("50", 18)
    );
    await expect(
      HEI.connect(minter).burnFrom(user.address, ethers.parseUnits("100", 18))
    )
      .to.be.revertedWithCustomError(HEI, "ERC20InsufficientAllowance")
      .withArgs(
        minter.address,
        ethers.parseUnits("50", 18),
        ethers.parseUnits("100", 18)
      );
  });
});
