This folder contains the dev keystore, it can be used together with Dockerfile.dev

How are bin files generated:

```bash
# subkey inspect '//Alice'
echo -n "e5be9a5092b81bca64be81d212e7f2f9eba183bb7a90954f7b76361f6edb5c0a" | xxd -r -p > rococo.bin
# internal foundry account
# 0x9965507D1a55bcC2695C58ba16FB37d819B0A4dc 8b3a350cf5c34c9194ca85829a2df0ec3153be0318b5e2d3348e872092edffba m/44'/60'/0'/0/5
echo -n "8b3a350cf5c34c9194ca85829a2df0ec3153be0318b5e2d3348e872092edffba" | xxd -r -p > sepolia.bin
```
