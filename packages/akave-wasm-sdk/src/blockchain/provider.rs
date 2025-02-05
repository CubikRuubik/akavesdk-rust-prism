use web3::{Transport, Web3};

pub struct BlockchainProvider<T>
where
    T: Transport,
{
    pub web3_provider: Web3<T>,
}
