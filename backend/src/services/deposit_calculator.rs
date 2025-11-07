const LAMPORTS_PER_SOL: u64 = 1_000_000_000;
const ESTIMATED_TRANSACTION_FEE: u64 = 5_000;
const PRIORITY_FEE_BUFFER: u64 = 10_000;
const TRANSACTIONS_PER_SESSION: u64 = 100;

pub struct DepositCalculator;

impl DepositCalculator {
    pub fn new() -> Self {
        DepositCalculator
    }

    pub fn calculate_deposit(&self, expected_transactions: u64) -> u64 {
        let base_fee = ESTIMATED_TRANSACTION_FEE * expected_transactions;
        let priority_buffer = PRIORITY_FEE_BUFFER * expected_transactions;
        base_fee + priority_buffer
    }

    pub fn calculate_default_deposit(&self) -> u64 {
        self.calculate_deposit(TRANSACTIONS_PER_SESSION)
    }

    pub fn should_topup(&self, current_balance: u64, threshold: u64) -> bool {
        current_balance < threshold
    }

    pub fn calculate_topup(&self, current_balance: u64, target_balance: u64) -> u64 {
        if current_balance > target_balance {
            return 0;
        }

        target_balance - current_balance
    }

    pub fn estimated_transaction_fee(&self) -> u64 {
        ESTIMATED_TRANSACTION_FEE * PRIORITY_FEE_BUFFER
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_deposit_amount() {
        let calculator = DepositCalculator::new();
        let amount = calculator.calculate_deposit(10);
        assert_eq!(amount, 150_000)
    }

    #[test]
    fn test_calculate_topup_amount() {
        let calculator = DepositCalculator::new();
        let topup_amount = calculator.calculate_topup(100_000, 1_000_000_000);
        assert_eq!(topup_amount, 999_900_000)
    }

    #[test]
    fn test_ahould_topup() {
        let calculator = DepositCalculator::new();
        assert!(calculator.should_topup(1_000_000_000, 500_000));
        assert!(calculator.should_topup(600_000, 1_000_000_000));
    }
}
