#import <Foundation/Foundation.h>
#import <Vision/Vision.h>
#import <CoreGraphics/CoreGraphics.h>
#import <ImageIO/ImageIO.h>
#include <stdlib.h>
#include <string.h>

int run_macos_vision_ocr(
    const uint8_t *image_bytes,
    size_t length,
    char **out_json
) {
    @autoreleasepool {
        if (!image_bytes || length == 0 || !out_json) {
            return -1;
        }

        NSData *data = [NSData dataWithBytesNoCopy:(void *)image_bytes length:length freeWhenDone:NO];
        CGImageSourceRef source = CGImageSourceCreateWithData((__bridge CFDataRef)data, NULL);
        if (!source) {
            return -2;
        }

        CGImageRef imageRef = CGImageSourceCreateImageAtIndex(source, 0, NULL);
        CFRelease(source);
        if (!imageRef) {
            return -3;
        }

        VNImageRequestHandler *handler = [[VNImageRequestHandler alloc] initWithCGImage:imageRef options:@{}];
        CGImageRelease(imageRef);

        VNRecognizeTextRequest *request = [[VNRecognizeTextRequest alloc] init];
        request.recognitionLevel = VNRequestTextRecognitionLevelAccurate;
        request.usesLanguageCorrection = YES;

        NSError *languageError = nil;
        NSArray<NSString *> *supported = [VNRecognizeTextRequest
            supportedRecognitionLanguagesForTextRecognitionLevel:request.recognitionLevel
            revision:request.revision
            error:&languageError];
        if (supported && !languageError) {
            NSMutableArray<NSString *> *languages = [NSMutableArray array];
            for (NSString *language in @[@"en-US", @"es-ES"]) {
                if ([supported containsObject:language]) {
                    [languages addObject:language];
                }
            }
            if (languages.count > 0) {
                request.recognitionLanguages = languages;
            }
        }

        NSError *error = nil;
        BOOL success = [handler performRequests:@[request] error:&error];
        if (!success || error) {
            return -4;
        }

        NSMutableArray *linesArray = [NSMutableArray array];
        NSRegularExpression *tokenPattern = [NSRegularExpression regularExpressionWithPattern:@"\\S+" options:0 error:&error];
        if (!tokenPattern || error) {
            return -7;
        }

        for (VNRecognizedTextObservation *observation in request.results) {
            NSArray<VNRecognizedText *> *candidates = [observation topCandidates:1];
            if (candidates.count == 0) {
                continue;
            }

            VNRecognizedText *topCandidate = candidates.firstObject;
            NSString *lineString = topCandidate.string;
            if (!lineString || lineString.length == 0) {
                continue;
            }

            // Apple Vision coordinates: origin (0,0) is at LOWER-LEFT.
            // Screen-Pin coordinates: origin (0,0) is at UPPER-LEFT.
            CGRect lineBox = observation.boundingBox;
            CGFloat lineY = 1.0 - (lineBox.origin.y + lineBox.size.height);

            NSMutableArray *wordsArray = [NSMutableArray array];

            for (NSTextCheckingResult *match in [tokenPattern matchesInString:lineString options:0 range:NSMakeRange(0, lineString.length)]) {
                NSRange substringRange = match.range;
                NSString *substring = [lineString substringWithRange:substringRange];
                NSError *boxError = nil;
                VNRectangleObservation *wordBoxObs = [topCandidate boundingBoxForRange:substringRange error:&boxError];
                if (wordBoxObs) {
                    CGRect wordBox = wordBoxObs.boundingBox;
                    CGFloat wordY = 1.0 - (wordBox.origin.y + wordBox.size.height);
                    [wordsArray addObject:@{
                        @"text": substring,
                        @"bounds": @{
                            @"x": @(wordBox.origin.x),
                            @"y": @(wordY),
                            @"width": @(wordBox.size.width),
                            @"height": @(wordBox.size.height)
                        }
                    }];
                } else {
                    CGFloat propX = lineBox.origin.x + (lineBox.size.width * (CGFloat)substringRange.location / (CGFloat)lineString.length);
                    CGFloat propW = lineBox.size.width * ((CGFloat)substringRange.length / (CGFloat)lineString.length);
                    [wordsArray addObject:@{
                        @"text": substring,
                        @"bounds": @{
                            @"x": @(propX),
                            @"y": @(lineY),
                            @"width": @(propW),
                            @"height": @(lineBox.size.height)
                        }
                    }];
                }
            }

            if (wordsArray.count > 0) {
                [linesArray addObject:@{
                    @"text": lineString,
                    @"words": wordsArray,
                    @"bounds": @{
                        @"x": @(lineBox.origin.x),
                        @"y": @(lineY),
                        @"width": @(lineBox.size.width),
                        @"height": @(lineBox.size.height)
                    }
                }];
            }
        }

        NSDictionary *resultDict = @{
            @"lines": linesArray
        };

        NSData *jsonData = [NSJSONSerialization dataWithJSONObject:resultDict options:0 error:&error];
        if (!jsonData || error) {
            return -5;
        }

        NSString *jsonString = [[NSString alloc] initWithData:jsonData encoding:NSUTF8StringEncoding];
        if (!jsonString) {
            return -6;
        }

        *out_json = strdup([jsonString UTF8String]);
        return 0;
    }
}

void free_macos_vision_ocr_result(char *ptr) {
    if (ptr) {
        free(ptr);
    }
}
