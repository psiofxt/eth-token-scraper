# eth-token-scraper
Intended to concurrently grab all holders of an ERC20 token and output the balances
they hold.

Set env var `CONTRACT_ADDRESS=""` to the ERC20 contract you wish to scrap and
`ETH_NETWORK=""` to specify the network.

Currently this is not optimized for contracts with greater than 50,000 events.
Optimizations in regards to concurrency limits based on the amount of events
remains to be done.

# Example
`CONTRACT_ADDRESS="0xcba0b17f1afa724d2a19c040d7f90f0468b662ea"` \
`ETH_NETWORK=rinkeby`

```
"0x26da0a36d60ef200"
"0x26da0a36d60ef200"
"0xae771444348cc00"
"0x1043561a882930000"
"0x298cf01478215600"
```
