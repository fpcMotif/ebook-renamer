#!/bin/bash
# Test the new citation formatting

mkdir -p /tmp/ebook_test_citations
cd /tmp/ebook_test_citations

# Create test files based on user's examples
touch "Differential Geometry (Paulo Ventura Araujo).pdf"
touch "Uncovering Quantum Field Theory and the Standard Model (Wolfgang Bietenholz, Uwe-Jens Wiese).pdf"
touch "Differential Geometry and General Relativity (Canbin Liang, Bin Zhou).pdf"
touch "Algebraic Topology - A Structural Introduction (Marco Grandis).pdf"
touch "A supplement for Category theory for computing science (Michael Barr, Charles Wells).pdf"
touch "Higher-Dimensional Categories (Cheng E., Lauda A.).pdf"
touch "Higher Dimensional Categories From Double To Multiple Categories (Marco, Grandis).pdf"
touch "Directed Algebraic Topology Models of Non-Reversible Worlds (Marco Grandis).pdf"
touch "Abelian Categories An Introduction to the Theory of Functors (Peter Freyd).pdf"
touch "Theory of Categories (Pure and Applied Mathematics (Academic Press)) (Barry Mitchell).pdf"
touch "Category Theory Course [Lecture notes] (John Baez).pdf"
touch "An introduction to homotopy theory via groupoids and universal constructions (Heath P.R.).pdf"
touch "Categories, Types, and Structures An Introduction to Category Theory for the Working Computer Scientist (Foundations of… (Andrea Asperti, Giuseppe Longo).pdf"

# Run the renamer
/Users/f/format/target/release/ebook_renamer . --dry-run --skip-cloud-hash

# Clean up
cd -
rm -rf /tmp/ebook_test_citations

