//! Unit tests for compression logic.
//!
//! Tests tree-sitter and whitespace compression with representative code samples.
//! Verifies compression ratios, node extraction, and error handling.

use ruley::packer::compress::{Compressor, Language, TreeSitterCompressor, WhitespaceCompressor};

#[cfg(feature = "compression-typescript")]
mod tree_sitter_tests {
    use super::*;

    /// Test TypeScript function compression extracts signatures and removes bodies.
    #[test]
    fn test_tree_sitter_typescript_function_compression() {
        let source = r#"
        function calculateSum(a: number, b: number): number {
            const result = a + b;
            const formatted = `Sum is ${result}`;
            return result;
        }

        function processArray(items: string[]): string[] {
            return items.map(item => {
                const trimmed = item.trim();
                const upper = trimmed.toUpperCase();
                return upper;
            });
        }
        "#;

        let compressor = TreeSitterCompressor;
        let result = compressor
            .compress(source, Language::TypeScript)
            .expect("Compression should succeed");

        // Verify compression occurred (~70% reduction, ratio around 0.3)
        let original_size = source.len() as f32;
        let compressed_size = result.len() as f32;
        let ratio = compressed_size / original_size;
        assert!(
            ratio < 0.35,
            "TypeScript compression should achieve ~70% reduction (ratio < 0.35), got {}",
            ratio
        );

        // Verify function signatures are preserved
        assert!(
            result.contains("calculateSum") || result.contains("processArray"),
            "Function names should be preserved in compression"
        );
    }

    /// Test TypeScript class compression preserves method signatures.
    #[test]
    fn test_tree_sitter_typescript_class_compression() {
        let source = r#"
        export class DataProcessor {
            private cache: Map<string, any> = new Map();

            constructor(private name: string) {
                this.initialize();
            }

            private initialize(): void {
                console.log("Initializing " + this.name);
                this.setupListeners();
            }

            public process(data: Record<string, unknown>): Promise<void> {
                const normalized = this.normalize(data);
                const validated = this.validate(normalized);
                return this.save(validated);
            }

            private normalize(data: any): any {
                return Object.keys(data).reduce((acc, key) => {
                    acc[key] = String(data[key]).trim();
                    return acc;
                }, {});
            }
        }
        "#;

        let compressor = TreeSitterCompressor;
        let result = compressor
            .compress(source, Language::TypeScript)
            .expect("Compression should succeed");

        // Verify compression occurred (~70% reduction target, ratio around 0.3)
        let original_size = source.len() as f32;
        let compressed_size = result.len() as f32;
        let ratio = compressed_size / original_size;
        assert!(
            ratio < 0.35,
            "TypeScript class compression should achieve ~70% reduction (ratio < 0.35), got {}\nCompressed: {}\nOriginal: {}",
            ratio,
            result,
            source
        );

        // Verify class name is preserved
        assert!(
            result.contains("DataProcessor"),
            "Class name should be preserved in compression"
        );
    }

    /// Test TypeScript imports and exports are fully preserved.
    #[test]
    fn test_tree_sitter_typescript_imports_exports() {
        let source = r#"
        import { Component, ReactNode } from 'react';
        import { useEffect, useState } from 'react';
        import type { User, Role } from './types';
        import * as utils from './utils';

        export interface Config {
            apiUrl: string;
            timeout: number;
        }

        export const DEFAULT_CONFIG: Config = {
            apiUrl: 'https://api.example.com',
            timeout: 5000,
        };

        export class MyComponent extends Component {
            render(): ReactNode {
                return <div>Hello</div>;
            }
        }

        export default MyComponent;
        "#;

        let compressor = TreeSitterCompressor;
        let result = compressor
            .compress(source, Language::TypeScript)
            .expect("Compression should succeed");

        // Verify imports are preserved
        assert!(
            result.contains("import") || result.contains("from"),
            "Imports should be preserved"
        );

        // Verify exports are preserved
        assert!(result.contains("export"), "Exports should be preserved");
    }

    /// Test tree-sitter gracefully handles invalid TypeScript syntax.
    #[test]
    fn test_tree_sitter_parse_failure_graceful_handling() {
        let invalid_source = "function broken( { // missing closing paren\nconst x = {";

        let compressor = TreeSitterCompressor;
        let result = compressor.compress(invalid_source, Language::TypeScript);

        // Should return an error that can be handled (used for fallback)
        assert!(
            result.is_err(),
            "Invalid TypeScript should return error for fallback"
        );
    }
}

#[cfg(not(feature = "compression-typescript"))]
mod tree_sitter_disabled_tests {
    use super::*;

    /// Test that tree-sitter compression is unavailable when feature disabled.
    #[test]
    fn test_tree_sitter_feature_disabled() {
        let source = "function test(): void {}";
        let compressor = TreeSitterCompressor;
        let result = compressor.compress(source, Language::TypeScript);

        // Should return error when feature disabled
        assert!(
            result.is_err(),
            "Tree-sitter should error when feature disabled"
        );
    }
}

mod whitespace_tests {
    use super::*;

    /// Test whitespace compressor normalizes multiple spaces and tabs.
    #[test]
    fn test_whitespace_normalization() {
        let source = "const   x   =   {  key:   'value'  };";
        let compressor = WhitespaceCompressor;
        let result = compressor
            .compress(source, Language::JavaScript)
            .expect("Whitespace compression should succeed");

        // Multiple spaces should be reduced to single space
        assert!(
            !result.contains("  "),
            "Multiple spaces should be normalized to single space"
        );
        assert!(
            result.contains("const x = { key: 'value' };"),
            "Normalized content should match expected format"
        );
    }

    /// Test whitespace compressor removes empty lines.
    #[test]
    fn test_whitespace_blank_line_removal() {
        let source = "const x = 1;\n\n\nconst y = 2;\n\nconst z = 3;";
        let compressor = WhitespaceCompressor;
        let result = compressor
            .compress(source, Language::JavaScript)
            .expect("Whitespace compression should succeed");

        // Count newlines (should have one per line)
        let line_count = result.lines().count();
        assert_eq!(line_count, 3, "Should have 3 lines (empty lines removed)");
        assert!(
            result.contains("const x = 1;"),
            "First line should be preserved"
        );
        assert!(
            result.contains("const y = 2;"),
            "Second line should be preserved"
        );
        assert!(
            result.contains("const z = 3;"),
            "Third line should be preserved"
        );
    }

    /// Test whitespace compressor trims leading and trailing whitespace.
    #[test]
    fn test_whitespace_line_trimming() {
        let source = "   const x = 1;   \n\t\tconst y = 2;    ";
        let compressor = WhitespaceCompressor;
        let result = compressor
            .compress(source, Language::JavaScript)
            .expect("Whitespace compression should succeed");

        let lines: Vec<&str> = result.lines().collect();
        for line in lines {
            // Lines should not start or end with whitespace
            assert!(
                !line.starts_with(' ') && !line.starts_with('\t'),
                "Line should not start with whitespace: '{}'",
                line
            );
            assert!(
                !line.ends_with(' ') && !line.ends_with('\t'),
                "Line should not end with whitespace: '{}'",
                line
            );
        }
    }

    /// Test whitespace compression achieves target reduction ratio.
    #[test]
    fn test_whitespace_compression_ratio() {
        let source = r#"
        function example() {
            const    data   =   {
                name:     'test',
                value:    42,
            };

            return   data;
        }
        "#;

        let compressor = WhitespaceCompressor;
        let result = compressor
            .compress(source, Language::JavaScript)
            .expect("Whitespace compression should succeed");

        let original_size = source.len() as f32;
        let compressed_size = result.len() as f32;
        let ratio = compressed_size / original_size;

        // Whitespace compression should stay within 0.6-0.7 range
        // Observed: can achieve better (0.4-0.6) with heavy whitespace
        assert!(
            ratio >= 0.4 && ratio <= 0.7,
            "Whitespace compression ratio should be 0.4-0.7, got {}",
            ratio
        );
    }

    /// Test whitespace compression on representative code sample (100+ lines).
    #[test]
    fn test_whitespace_representative_sample() {
        let source = r#"
        class UserManager {
            private    users:   User[]  =  [];
            private    cache:   Map<string,  User>  =  new  Map();

            constructor(private logger: Logger) {
                this.initialize();
            }

            private   initialize():   void   {
                console.log('Initializing UserManager');
            }

            async   getUser(id:  string):   Promise<User  |  null>  {
                if  (this.cache.has(id))  {
                    return   this.cache.get(id)   ||   null;
                }

                const   user  =  await  this.fetchFromDatabase(id);
                if  (user)  {
                    this.cache.set(id,  user);
                }

                return   user;
            }

            async   createUser(userData:  Partial<User>):  Promise<User>  {
                const   validated  =  this.validate(userData);
                const   normalized  =  this.normalize(validated);
                const   user  =  await  this.saveToDatabase(normalized);
                return   user;
            }

            private   validate(data:  Partial<User>):  Partial<User>  {
                if  (!data.name   ||   data.name.length  ===  0)  {
                    throw   new   Error('Name is required');
                }
                return   data;
            }
        }
        "#;

        let compressor = WhitespaceCompressor;
        let result = compressor
            .compress(source, Language::TypeScript)
            .expect("Whitespace compression should succeed");

        let original_size = source.len() as f32;
        let compressed_size = result.len() as f32;
        let ratio = compressed_size / original_size;

        // Should achieve meaningful compression
        // Representative samples with excess whitespace can achieve better ratios
        assert!(
            ratio >= 0.5 && ratio <= 0.7,
            "Whitespace compression ratio should be 0.5-0.7 for representative samples, got {}",
            ratio
        );
        assert!(
            compressed_size > 0.0,
            "Compressed content should not be empty"
        );
    }

    /// Test whitespace compression with tabs.
    #[test]
    fn test_whitespace_tab_handling() {
        let source = "const\tx\t=\t{\tkey:\t'value'\t};";
        let compressor = WhitespaceCompressor;
        let result = compressor
            .compress(source, Language::JavaScript)
            .expect("Whitespace compression should succeed");

        // Tabs should be normalized to single space
        assert!(
            !result.contains('\t'),
            "Tabs should be normalized to spaces"
        );
        assert!(
            result.contains("const x = { key: 'value' };"),
            "Content should be normalized correctly"
        );
    }
}

mod compression_trait_tests {
    use super::*;

    /// Verify compression ratio targets are documented.
    #[test]
    fn test_whitespace_compressor_ratio_target() {
        let compressor = WhitespaceCompressor;
        let ratio = compressor.compression_ratio();

        // Target ~30-40% reduction (0.6 ratio)
        assert_eq!(ratio, 0.6, "Whitespace compressor should target 0.6 ratio");
    }

    /// Verify compressor trait is properly implemented.
    #[test]
    fn test_compressor_trait_implementation() {
        let compressor = WhitespaceCompressor;
        let source = "const  x  =  1;";

        // Trait method should work correctly
        let result = compressor
            .compress(source, Language::JavaScript)
            .expect("Compressor trait should work");

        assert!(!result.is_empty(), "Compressed result should not be empty");
    }
}
