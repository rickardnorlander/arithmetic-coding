use std::io;

use crate::Model;
use bitstream_io::BitWrite;

pub struct Encoder<M>
where
    M: Model,
{
    model: M,
    precision: u32,
    low: u32,
    high: u32,
    pending: usize,
}

impl<M> Encoder<M>
where
    M: Model,
{
    pub fn new(model: M, precision: u32) -> Self {
        let low = 0;
        let high = 2u32.pow(precision);
        let pending = 0;

        Self {
            model,
            precision,
            low,
            high,
            pending,
        }
    }

    const fn half(&self) -> u32 {
        2u32.pow(self.precision - 1)
    }

    const fn quarter(&self) -> u32 {
        2u32.pow(self.precision - 2)
    }

    fn encode_symbol<W: BitWrite>(
        &mut self,
        symbol: Option<&M::Symbol>,
        output: &mut W,
    ) -> io::Result<()> {
        let range = self.high - self.low + 1;
        let p = self.model.probability(symbol);

        self.high = self.low + (range * p.end) / M::denominator();
        self.low += (range * p.start) / M::denominator();
        self.model.update(symbol);
        self.normalise(output)
    }

    fn normalise<W: BitWrite>(&mut self, output: &mut W) -> io::Result<()> {
        // this algorithm is derived from this article - https://marknelson.us/posts/2014/10/19/data-compression-with-arithmetic-coding.html

        while self.high < self.half() || self.low > self.half() {
            if self.high < self.half() {
                self.emit(false, output)?;
                self.high *= 2;
                self.low *= 2;
            } else {
                // self.low >= self.half()
                self.emit(true, output)?;
                self.low = 2 * (self.low - self.half());
                self.high = 2 * (self.high - self.half());
            }
        }

        while self.quarter() < self.low && self.high < 3 * self.quarter() {
            self.pending += 1;
            self.low = 2 * (self.low - self.quarter());
            self.high = 2 * (self.high - self.quarter());
        }

        Ok(())
    }

    fn emit<W: BitWrite>(&mut self, bit: bool, output: &mut W) -> io::Result<()> {
        output.write_bit(bit)?;
        for _ in 0..self.pending {
            output.write_bit(!bit)?;
        }
        self.pending = 0;
        Ok(())
    }

    fn flush<W: BitWrite>(&mut self, output: &mut W) -> io::Result<()> {
        self.pending += 1;
        if self.low <= self.quarter() {
            self.emit(false, output)?;
        } else {
            self.emit(true, output)?;
        }

        Ok(())
    }

    pub fn encode<W: BitWrite>(
        &mut self,
        symbols: impl IntoIterator<Item = M::Symbol>,
        output: &mut W,
    ) -> io::Result<()> {
        for symbol in symbols {
            self.encode_symbol(Some(&symbol), output)?;
        }
        self.encode_symbol(None, output)?;
        self.flush(output)?;
        Ok(())
    }
}
