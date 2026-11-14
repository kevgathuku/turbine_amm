use anchor_lang::error_code;
use constant_product_curve::CurveError;

#[error_code]
pub enum AmmError {
    #[msg("fee percentage can only be between 0 to 100 (10,000 basis points)")]
    FeePercentErr,
}
