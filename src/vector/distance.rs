use wide::f32x16;

pub fn cosine_simd(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len().min(b.len());
    let chunks = len / 16;
    let remainder = len % 16;

    let mut dot = f32x16::splat(0.0);
    let mut norm_a = f32x16::splat(0.0);
    let mut norm_b = f32x16::splat(0.0);

    for i in 0..chunks {
        let offset = i * 16;
        let end = offset + 16;
        let mut a_arr = [0.0f32; 16];
        let mut b_arr = [0.0f32; 16];
        a_arr.copy_from_slice(&a[offset..end]);
        b_arr.copy_from_slice(&b[offset..end]);
        let av = f32x16::new(a_arr);
        let bv = f32x16::new(b_arr);
        dot += av * bv;
        norm_a += av * av;
        norm_b += bv * bv;
    }

    let mut dot_sum = dot.reduce_add();
    let mut norm_a_sum = norm_a.reduce_add();
    let mut norm_b_sum = norm_b.reduce_add();

    let start = chunks * 16;
    for i in 0..remainder {
        let ai = a[start + i];
        let bi = b[start + i];
        dot_sum += ai * bi;
        norm_a_sum += ai * ai;
        norm_b_sum += bi * bi;
    }

    let denominator = (norm_a_sum.sqrt() * norm_b_sum.sqrt()).max(1e-10);
    dot_sum / denominator
}

pub fn cosine_scalar(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len().min(b.len());
    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;

    for i in 0..len {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    let denominator = (norm_a.sqrt() * norm_b.sqrt()).max(1e-10);
    dot / denominator
}

pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() < 32 && b.len() < 32 {
        cosine_scalar(a, b)
    } else {
        cosine_simd(a, b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_identical() {
        let v = vec![1.0f32; 384];
        let sim = cosine(&v, &v);
        assert!((sim - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_orthogonal() {
        let a = vec![1.0, 0.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0, 0.0];
        let sim = cosine(&a, &b);
        assert!(sim.abs() < 0.001);
    }

    #[test]
    fn test_simd_scalar_agree() {
        let a: Vec<f32> = (0..384).map(|i| (i as f32).sin()).collect();
        let b: Vec<f32> = (0..384).map(|i| (i as f32).cos()).collect();
        let simd = cosine_simd(&a, &b);
        let scalar = cosine_scalar(&a, &b);
        assert!((simd - scalar).abs() < 0.001);
    }

    #[test]
    fn test_cosine_opposite() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![-1.0, -2.0, -3.0];
        let sim = cosine(&a, &b);
        assert!((sim + 1.0).abs() < 0.001);
    }
}
